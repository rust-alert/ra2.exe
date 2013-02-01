//! 原生 GUI 入口：二进制名 `ra2`（Windows 上为 `ra2.exe`）。
//!
//! 不是命令行工具——启动配置来自 exe/工作目录旁的 `config.toml`，然后由窗口接管进程。

#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod config;
mod fs_source;

use std::sync::Arc;

use ra_adaptor::{detect_edition, find_ci_file};
use ra_assets::{Palette, ShpFile};
use ra_map::MapInfo;
use ra_renderer::{Renderer, RgbaImage};
use ra_rules::load_rules;
use ra_types::{GameEdition, RaError, RaResult};
use ra_world::World;
use winit::application::ApplicationHandler;
use winit::event::WindowEvent;
use winit::event_loop::{ActiveEventLoop, ControlFlow, EventLoop};
use winit::window::{Window, WindowId};

use crate::config::DesktopConfig;
use crate::fs_source::GameAssetSource;

struct App {
    window: Option<Arc<Window>>,
    title_base: String,
    boot_note: String,
    world: Option<World>,
    renderer: Renderer,
}

impl App {
    fn new(boot_note: String, world: Option<World>, preview: Option<RgbaImage>) -> Self {
        let edition = world
            .as_ref()
            .map(|w| w.edition.as_str())
            .unwrap_or("—");
        let mut renderer = Renderer::new();
        if let Some(image) = preview {
            renderer.set_preview(image);
        }
        Self {
            window: None,
            title_base: format!("ra2 ({edition})"),
            boot_note,
            world,
            renderer,
        }
    }

    fn refresh_title(&self) {
        if let Some(window) = &self.window {
            let tick = self.world.as_ref().map(|w| w.tick).unwrap_or(0);
            window.set_title(&format!(
                "{} · {} · t{}",
                self.title_base, self.boot_note, tick
            ));
        }
    }
}

impl ApplicationHandler for App {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.window.is_some() {
            return;
        }
        let window = Arc::new(
            event_loop
                .create_window(
                    Window::default_attributes()
                        .with_title(self.title_base.clone())
                        .with_inner_size(winit::dpi::LogicalSize::new(1024.0, 768.0)),
                )
                .expect("创建窗口失败"),
        );
        if let Err(e) = self.renderer.attach_window(window.clone()) {
            eprintln!("ra2 wgpu: {e}");
        } else {
            eprintln!(
                "ra2 gpu: {} · preview={}",
                self.renderer.backend_name(),
                if self.renderer.has_preview() {
                    "yes"
                } else {
                    "no"
                }
            );
        }
        self.window = Some(window);
        self.refresh_title();
    }

    fn window_event(&mut self, event_loop: &ActiveEventLoop, _id: WindowId, event: WindowEvent) {
        match event {
            WindowEvent::CloseRequested => event_loop.exit(),
            WindowEvent::Resized(size) => {
                self.renderer.resize(size.width, size.height);
            }
            WindowEvent::RedrawRequested => {
                if let Some(world) = self.world.as_mut() {
                    world.advance_tick();
                }
                self.renderer.draw_frame(self.world.as_ref());
                self.refresh_title();
                if let Some(window) = &self.window {
                    window.request_redraw();
                }
            }
            _ => {}
        }
    }

    fn about_to_wait(&mut self, _event_loop: &ActiveEventLoop) {
        if let Some(window) = &self.window {
            window.request_redraw();
        }
    }
}

struct BootResult {
    note: String,
    world: Option<World>,
    preview: Option<RgbaImage>,
}

fn load_preview_sprite(source: &GameAssetSource) -> Option<(String, RgbaImage)> {
    let pal_bytes = source.vfs.read("unittem.pal")?;
    let pal = Palette::parse(&pal_bytes).ok()?;
    let candidates = [
        "mouse.shp",
        "e1.shp",
        "clock.shp",
        "power.shp",
        "gaairc.shp",
    ];
    for name in candidates {
        let Some(bytes) = source.vfs.read(name) else {
            continue;
        };
        let Ok(shp) = ShpFile::parse(&bytes) else {
            continue;
        };
        let Some(frame) = shp.frames.first() else {
            continue;
        };
        if frame.frame_width == 0 || frame.frame_height == 0 {
            continue;
        }
        let rgba = frame.to_rgba(&pal);
        let image = RgbaImage::new(
            u32::from(frame.frame_width),
            u32::from(frame.frame_height),
            rgba,
        )?;
        return Some((name.to_string(), image));
    }
    None
}

fn boot_world(cfg: &DesktopConfig) -> RaResult<BootResult> {
    let root = cfg.game_dir();
    let explicit = match cfg.edition.as_deref() {
        Some(s) => Some(GameEdition::parse(s)?),
        None => None,
    };
    let manifest = detect_edition(&root, explicit)?;
    let chain = &manifest.chain;

    let mut source = GameAssetSource::new(manifest.root.clone());

    let mut mounted_root = 0usize;
    let mut skipped_root = 0usize;
    for name in &manifest.present_mixes {
        let Some(path) = find_ci_file(&manifest.root, name) else {
            continue;
        };
        let data = std::fs::read(&path)
            .map_err(|e| RaError::Io(format!("{}: {e}", path.display())))?;
        match source.vfs.mount_bytes(name.clone(), data) {
            Ok(()) => mounted_root += 1,
            Err(_) => skipped_root += 1,
        }
    }

    let mut mounted_nested = 0usize;
    for name in chain.nested_mix_files {
        match source.vfs.mount_nested(name) {
            Ok(true) => mounted_nested += 1,
            Ok(false) => {}
            Err(_) => {}
        }
    }

    let mut note = format!(
        "{} · 根mix {} · 嵌套 {} · 跳过 {} · 缺盘 {}",
        chain.edition.as_str(),
        mounted_root,
        mounted_nested,
        skipped_root,
        manifest.missing_mixes.len()
    );

    let preview = match load_preview_sprite(&source) {
        Some((name, image)) => {
            note = format!("{note} · shp:{name}");
            Some(image)
        }
        None => {
            note = format!("{note} · shp:无");
            None
        }
    };

    let world = match load_rules(&source, chain.edition) {
        Ok(rules) => {
            let sections = rules.rules.sections.len();
            note = format!("{note} · rules#{sections}");
            let map = MapInfo::empty(chain.edition, "boot");
            Some(World::new(chain.edition, &rules, map))
        }
        Err(e) => {
            note = format!("{note} · 规则待加载（{e}）");
            None
        }
    };

    Ok(BootResult {
        note,
        world,
        preview,
    })
}

fn main() {
    if let Err(e) = run() {
        eprintln!("ra2 错误: {e}");
        std::process::exit(1);
    }
}

fn run() -> RaResult<()> {
    let cfg = DesktopConfig::load_or_default();
    let boot = match boot_world(&cfg) {
        Ok(v) => v,
        Err(e) => BootResult {
            note: format!("启动失败: {e}"),
            world: None,
            preview: None,
        },
    };
    eprintln!(
        "ra2 boot: {} · world={}",
        boot.note,
        if boot.world.is_some() { "ok" } else { "none" }
    );

    let event_loop = EventLoop::new().map_err(|e| RaError::Msg(e.to_string()))?;
    event_loop.set_control_flow(ControlFlow::Poll);

    let mut app = App::new(boot.note, boot.world, boot.preview);
    event_loop
        .run_app(&mut app)
        .map_err(|e| RaError::Msg(e.to_string()))?;
    Ok(())
}
