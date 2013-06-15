//! 原生 GUI 入口：二进制名 `ra2`（Windows 上为 `ra2.exe`）。
//!
//! 不是命令行工具——启动配置来自 exe/工作目录旁的 `config.toml`，然后由窗口接管进程。

#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod config;
mod fs_source;

use std::collections::HashMap;
use std::sync::Arc;
use std::time::Instant;

use ra_adaptor::{detect_edition, ResourceChain};
use ra_assets::{
    rasterize_vxl_layer_poses, HvaFile, IniDocument, Palette, ShpFile, TmpFile, VplFile, VxlFile,
    VxlLayerPose,
};
use ra_logger;
use ra_map::{
    compose_terrain_preview, mount_theater_mixes, new_theater_shp_name, paint_cell_sprites,
    paint_map_overlays, parse_tileset_ini, seal_pass_grid_from_tmp, theater_ini_name,
    theater_palette, theater_tmp_extension, try_parse_boot_map, BOOT_MAP_CANDIDATES, MapEntityKind,
    MapInfo, Theater, TileBlit, TILE_HEIGHT, TILE_WIDTH,
};
use ra_renderer::{Renderer, RgbaImage};
use ra_rules::{load_rules_chain, ColorSchemes, OverlayTypeRegistry};
use ra_session::Session;
use ra_types::{AssetSource, GameEdition, RaError, RaResult};
use ra_world::World;
use winit::application::ApplicationHandler;
use winit::event::{ElementState, MouseButton, MouseScrollDelta, WindowEvent};
use winit::event_loop::{ActiveEventLoop, ControlFlow, EventLoop};
use winit::keyboard::{KeyCode, PhysicalKey};
use winit::window::{Window, WindowId};

use crate::config::{load_desktop_config_with_diagnostics, DesktopConfig};
use crate::fs_source::GameAssetSource;

struct App {
    window: Option<Arc<Window>>,
    title_base: String,
    session: Option<Session>,
    renderer: Renderer,
    /// 左键拖拽中：上一帧光标位置。
    drag_last: Option<(f64, f64)>,
    /// 已按下左键，等待第一次 CursorMoved 建立起点。
    drag_armed: bool,
    /// 本次左键按下后累计拖拽距离（像素）；用于区分点击与平移。
    drag_distance: f32,
    /// 最近光标位置（窗口像素）。
    cursor: (f64, f64),
    /// 上一帧时间，用于固定仿真时钟。
    last_pump: Instant,
    /// 已记录过的胜负文案，避免每帧刷日志。
    logged_outcome: Option<String>,
    /// Shift 是否按下（多选）。
    shift_down: bool,
    /// Ctrl 是否按下（全选同阵营等）。
    ctrl_down: bool,
}

impl App {
    fn new(_boot_note: String, session: Option<Session>, preview: Option<RgbaImage>) -> Self {
        let edition = session
            .as_ref()
            .map(|s| s.world.edition.as_str())
            .unwrap_or("—");
        let mut renderer = Renderer::new();
        if let Some(image) = preview {
            renderer.set_preview(image);
        }
        Self {
            window: None,
            title_base: format!("ra2 ({edition})"),
            session,
            renderer,
            drag_last: None,
            drag_armed: false,
            drag_distance: 0.0,
            cursor: (0.0, 0.0),
            last_pump: Instant::now(),
            logged_outcome: None,
            shift_down: false,
            ctrl_down: false,
        }
    }

    fn cursor_cell(&self) -> Option<(u16, u16)> {
        let session = self.session.as_ref()?;
        let window = self.window.as_ref()?;
        let size = window.inner_size();
        let (wx, wy) = self.renderer.camera().screen_to_world(
            self.cursor.0 as f32,
            self.cursor.1 as f32,
            size.width as f32,
            size.height as f32,
        );
        session.image_to_cell(wx, wy)
    }

    fn handle_left_click(&mut self) {
        let add = self.shift_down;
        let Some(cell) = self.cursor_cell() else {
            if !add {
                if let Some(session) = self.session.as_mut() {
                    session.selected.clear();
                }
            }
            return;
        };
        let Some(session) = self.session.as_mut() else {
            return;
        };
        if let Some(i) = session.pick_mobile_at(cell.0, cell.1) {
            if add {
                session.select_add(i);
                ra_logger::info(format!(
                    "加选实体 #{i} @({},{}) · 选中 {:?}",
                    cell.0, cell.1, session.selected
                ));
            } else {
                session.select_only(i);
                ra_logger::info(format!("选中实体 #{i} @({},{})", cell.0, cell.1));
            }
        } else if !add {
            session.selected.clear();
            ra_logger::debug(format!("点空地 ({},{})，清空选中", cell.0, cell.1));
        }
    }

    fn handle_right_click(&mut self) {
        let Some(cell) = self.cursor_cell() else {
            return;
        };
        let Some(session) = self.session.as_mut() else {
            return;
        };
        if session.selected.is_empty() {
            return;
        }
        if let Some(target) = session.pick_mobile_at(cell.0, cell.1) {
            let hostile = session
                .selected
                .first()
                .and_then(|&atk| {
                    let a = session.world.entities.get(atk)?;
                    let t = session.world.entities.get(target)?;
                    Some(a.owner != t.owner)
                })
                .unwrap_or(false);
            if hostile {
                ra_logger::info(format!(
                    "命令攻击 → #{target}（选中 {:?}）",
                    session.selected
                ));
                session.order_selected_attack(target);
                return;
            }
        }
        ra_logger::info(format!(
            "命令移动 → ({},{})（选中 {:?}）",
            cell.0, cell.1, session.selected
        ));
        session.order_selected_move(cell.0, cell.1);
    }

    fn refresh_title(&self) {
        if let Some(window) = &self.window {
            let tick = self.session.as_ref().map(|s| s.world.tick).unwrap_or(0);
            let sel = self
                .session
                .as_ref()
                .and_then(|s| s.selected.first().copied());
            let zoom = self.renderer.camera().zoom;
            let outcome = self.session.as_ref().and_then(|s| s.outcome.as_ref());
            let title = if let Some(ra_session::MatchOutcome::Victory { owner }) = outcome {
                format!("{} · t{} · 胜 {}", self.title_base, tick, owner)
            } else {
                let nsel = self.session.as_ref().map(|s| s.selected.len()).unwrap_or(0);
                match (sel, nsel) {
                    (Some(i), n) if n > 1 => {
                        format!("{} · t{} · #{i}+{} · z{:.2}", self.title_base, tick, n - 1, zoom)
                    }
                    (Some(i), _) => {
                        format!("{} · t{} · #{i} · z{:.2}", self.title_base, tick, zoom)
                    }
                    (None, _) => format!("{} · t{} · z{:.2}", self.title_base, tick, zoom),
                }
            };
            window.set_title(&title);
        }
    }

    fn note_outcome_once(&mut self) {
        let Some(session) = self.session.as_ref() else {
            return;
        };
        let Some(ra_session::MatchOutcome::Victory { owner }) = session.outcome.as_ref() else {
            return;
        };
        if self.logged_outcome.as_deref() == Some(owner.as_str()) {
            return;
        }
        self.logged_outcome = Some(owner.clone());
        ra_logger::info(format!("对局结束 · 胜方 {owner} · tick={}", session.world.tick));
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
            ra_logger::error(format!("wgpu 附着失败: {e}"));
        } else {
            ra_logger::info(format!(
                "gpu={} preview={} zoom={:.2}",
                self.renderer.backend_name(),
                if self.renderer.has_preview() {
                    "yes"
                } else {
                    "no"
                },
                self.renderer.camera().zoom
            ));
        }
        self.window = Some(window);
        self.refresh_title();
    }

    fn window_event(&mut self, event_loop: &ActiveEventLoop, _id: WindowId, event: WindowEvent) {
        match event {
            WindowEvent::CloseRequested => event_loop.exit(),
            WindowEvent::ModifiersChanged(mods) => {
                self.shift_down = mods.state().shift_key();
                self.ctrl_down = mods.state().control_key();
            }
            WindowEvent::Resized(size) => {
                self.renderer.resize(size.width, size.height);
            }
            WindowEvent::MouseInput {
                state,
                button: MouseButton::Left,
                ..
            } => match state {
                ElementState::Pressed => {
                    self.drag_armed = true;
                    self.drag_last = None;
                    self.drag_distance = 0.0;
                }
                ElementState::Released => {
                    let was_click = self.drag_armed && self.drag_distance < 6.0;
                    self.drag_armed = false;
                    self.drag_last = None;
                    if was_click {
                        self.handle_left_click();
                    }
                }
            },
            WindowEvent::MouseInput {
                state: ElementState::Pressed,
                button: MouseButton::Right,
                ..
            } => {
                self.handle_right_click();
            },
            WindowEvent::CursorMoved { position, .. } => {
                self.cursor = (position.x, position.y);
                if self.drag_armed {
                    if let Some((lx, ly)) = self.drag_last {
                        let dx = (position.x - lx) as f32;
                        let dy = (position.y - ly) as f32;
                        self.drag_distance += (dx * dx + dy * dy).sqrt();
                        self.renderer.pan_screen(dx, dy);
                    }
                    self.drag_last = Some((position.x, position.y));
                }
            },
            WindowEvent::MouseWheel { delta, .. } => {
                let steps = match delta {
                    MouseScrollDelta::LineDelta(_, y) => y,
                    MouseScrollDelta::PixelDelta(p) => p.y as f32 / 40.0,
                };
                if steps != 0.0 {
                    let factor = if steps > 0.0 { 1.1_f32 } else { 1.0 / 1.1 };
                    self.renderer.zoom_by(factor.powf(steps.abs()));
                }
            },
            WindowEvent::KeyboardInput { event, .. } => {
                if event.state != ElementState::Pressed {
                    return;
                }
                let step = 48.0_f32;
                match event.physical_key {
                    PhysicalKey::Code(KeyCode::KeyA) if self.ctrl_down => {
                        if let Some(session) = self.session.as_mut() {
                            let seed = session
                                .selected
                                .first()
                                .copied()
                                .or_else(|| {
                                    session.world.entities.iter().position(|e| {
                                        !e.dead
                                            && matches!(
                                                e.kind,
                                                MapEntityKind::Unit
                                                    | MapEntityKind::Infantry
                                                    | MapEntityKind::Aircraft
                                            )
                                    })
                                });
                            if let Some(i) = seed {
                                session.select_all_of_owner(i);
                                ra_logger::info(format!(
                                    "全选同阵营 · {} 个",
                                    session.selected.len()
                                ));
                            }
                        }
                    }
                    PhysicalKey::Code(KeyCode::ArrowLeft) | PhysicalKey::Code(KeyCode::KeyA) => {
                        self.renderer.pan_screen(step, 0.0);
                    }
                    PhysicalKey::Code(KeyCode::ArrowRight) | PhysicalKey::Code(KeyCode::KeyD) => {
                        self.renderer.pan_screen(-step, 0.0);
                    }
                    PhysicalKey::Code(KeyCode::ArrowUp) | PhysicalKey::Code(KeyCode::KeyW) => {
                        self.renderer.pan_screen(0.0, step);
                    }
                    PhysicalKey::Code(KeyCode::ArrowDown) | PhysicalKey::Code(KeyCode::KeyS) => {
                        self.renderer.pan_screen(0.0, -step);
                    }
                    PhysicalKey::Code(KeyCode::Equal) | PhysicalKey::Code(KeyCode::NumpadAdd) => {
                        self.renderer.zoom_by(1.1);
                    }
                    PhysicalKey::Code(KeyCode::Minus) | PhysicalKey::Code(KeyCode::NumpadSubtract) => {
                        self.renderer.zoom_by(1.0 / 1.1);
                    }
                    PhysicalKey::Code(KeyCode::Tab) => {
                        if let Some(session) = self.session.as_mut() {
                            session.cycle_selection();
                        }
                    }
                    PhysicalKey::Code(KeyCode::KeyF) => {
                        if let Some(session) = self.session.as_mut() {
                            if let Some(&atk) = session.selected.first() {
                                if let Some(tgt) = session.nearest_hostile(atk) {
                                    session.order_selected_attack(tgt);
                                }
                            }
                        }
                    }
                    _ => {}
                }
            }
            WindowEvent::RedrawRequested => {
                let now = Instant::now();
                let dt = now.duration_since(self.last_pump).as_secs_f64();
                self.last_pump = now;
                if let Some(session) = self.session.as_mut() {
                    let _advanced = session.pump(dt);
                }
                self.note_outcome_once();
                let snap = self.session.as_ref().map(|s| s.snapshot());
                self.renderer.draw_frame(snap.as_ref());
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
    session: Option<Session>,
    preview: Option<RgbaImage>,
}

fn load_map_terrain_preview(
    source: &GameAssetSource,
    map: &MapInfo,
    chain: &ResourceChain,
) -> Option<(String, RgbaImage, i32, i32)> {
    let mut image = compose_terrain_preview(source, map)?;
    let mut z_lookup: HashMap<(u16, u16), u8> = HashMap::new();
    for cell in &map.cells {
        z_lookup.insert((cell.x as u16, cell.y as u16), cell.z);
    }
    let overlay_registry = source
        .read(chain.rules_ini)
        .ok()
        .and_then(|b| IniDocument::parse(&b).ok())
        .map(|doc| OverlayTypeRegistry::from_rules(&doc))
        .unwrap_or_default();
    let (overlay_shp, overlay_mark) = paint_map_overlays(
        source,
        map,
        &mut image,
        chain.art_ini,
        &|id| overlay_registry.name(id).map(str::to_owned),
    );
    let terrain_painted = paint_terrain_objects(source, map, &mut image, &z_lookup);
    let color_rules = load_color_rules(source, chain);
    let structure_painted =
        paint_structure_entities(source, map, &mut image, &z_lookup, color_rules.as_ref());
    let mobile_painted =
        paint_mobile_entities(source, map, &mut image, &z_lookup, color_rules.as_ref());
    let rgba = RgbaImage::new(image.width, image.height, image.pixels)?;
    Some((
        format!(
            "map:{} cells={} drawn={} overlay#{} shp#{} mark#{} terrain_shp#{} struct_shp#{} mobile_shp#{} {}x{}",
            map.name,
            map.cells.len(),
            image.drawn,
            map.overlays.len(),
            overlay_shp,
            overlay_mark,
            terrain_painted,
            structure_painted,
            mobile_painted,
            rgba.width,
            rgba.height
        ),
        rgba,
        image.origin_x,
        image.origin_y,
    ))
}

/// 按 art / 剧院扩展名加载地形物件 SHP，叠到合成图上。
fn paint_terrain_objects(
    source: &GameAssetSource,
    map: &MapInfo,
    image: &mut ra_map::TerrainImage,
    z_lookup: &HashMap<(u16, u16), u8>,
) -> usize {
    if map.terrain_objects.is_empty() {
        return 0;
    }
    let art = source
        .vfs
        .read("art.ini")
        .and_then(|b| IniDocument::parse(&b).ok());
    let obj_pal = source
        .vfs
        .read("unittem.pal")
        .and_then(|b| Palette::parse(&b).ok())
        .or_else(|| {
            source
                .vfs
                .read(theater_palette(map.theater))
                .and_then(|b| Palette::parse(&b).ok())
        });
    let Some(obj_pal) = obj_pal else {
        return 0;
    };
    let ext = theater_tmp_extension(map.theater);
    let mut shp_cache: HashMap<String, ShpFile> = HashMap::new();
    let mut blit_cache: HashMap<String, TileBlit> = HashMap::new();
    let mut items: Vec<(u16, u16, TileBlit)> = Vec::new();

    for obj in &map.terrain_objects {
        let image_key = art
            .as_ref()
            .and_then(|a| a.get(&obj.name, "Image"))
            .unwrap_or(obj.name.as_str())
            .to_ascii_uppercase();
        if let Some(blit) = blit_cache.get(&image_key) {
            items.push((obj.x, obj.y, blit.clone()));
            continue;
        }
        let file = format!("{}.{ext}", image_key.to_ascii_lowercase());
        if !shp_cache.contains_key(&file) {
            let Some(bytes) = source.vfs.read(&file) else {
                continue;
            };
            let Ok(shp) = ShpFile::parse(&bytes) else {
                continue;
            };
            shp_cache.insert(file.clone(), shp);
        }
        let Some(shp) = shp_cache.get(&file) else {
            continue;
        };
        let Some(frame) = shp.frames.first() else {
            continue;
        };
        if frame.frame_width == 0 || frame.frame_height == 0 {
            continue;
        }
        let blit = TileBlit {
            width: u32::from(frame.frame_width),
            height: u32::from(frame.frame_height),
            offset_x: i32::from(frame.frame_x),
            offset_y: i32::from(frame.frame_y),
            rgba: frame.to_rgba(&obj_pal),
        };
        blit_cache.insert(image_key, blit.clone());
        items.push((obj.x, obj.y, blit));
    }

    paint_cell_sprites(image, &items, |x, y| {
        z_lookup.get(&(x, y)).copied().unwrap_or(0)
    })
}

/// 叠画 `[Structures]`：优先 `NewTheater` 文件名，否则普通 `.shp`。
fn load_color_rules(
    source: &GameAssetSource,
    chain: &ResourceChain,
) -> Option<(IniDocument, ColorSchemes)> {
    let bytes = source.vfs.read(chain.rules_ini)?;
    let doc = IniDocument::parse(&bytes).ok()?;
    let schemes = ColorSchemes::from_rules(&doc);
    Some((doc, schemes))
}

fn palette_for_owner(
    base: &Palette,
    owner: &str,
    color_rules: Option<&(IniDocument, ColorSchemes)>,
) -> Palette {
    if let Some((doc, schemes)) = color_rules {
        if let Some(hsv) = schemes.hsv_for_house(doc, owner) {
            return base.with_hsv_remap(hsv);
        }
    }
    base.for_owner(owner)
}

fn paint_structure_entities(
    source: &GameAssetSource,
    map: &MapInfo,
    image: &mut ra_map::TerrainImage,
    z_lookup: &HashMap<(u16, u16), u8>,
    color_rules: Option<&(IniDocument, ColorSchemes)>,
) -> usize {
    let structures: Vec<_> = map
        .entities
        .iter()
        .filter(|e| e.kind == MapEntityKind::Structure)
        .collect();
    if structures.is_empty() {
        return 0;
    }
    let art = source
        .vfs
        .read("art.ini")
        .and_then(|b| IniDocument::parse(&b).ok());
    let obj_pal = source
        .vfs
        .read("unittem.pal")
        .and_then(|b| Palette::parse(&b).ok())
        .or_else(|| {
            source
                .vfs
                .read(theater_palette(map.theater))
                .and_then(|b| Palette::parse(&b).ok())
        });
    let Some(obj_pal) = obj_pal else {
        return 0;
    };

    let mut shp_cache: HashMap<String, ShpFile> = HashMap::new();
    let mut blit_cache: HashMap<(String, String), TileBlit> = HashMap::new();
    let mut items: Vec<(u16, u16, TileBlit)> = Vec::new();

    for ent in structures {
        let image_key = art
            .as_ref()
            .and_then(|a| a.get(&ent.type_id, "Image"))
            .unwrap_or(ent.type_id.as_str())
            .to_ascii_uppercase();
        let cache_key = (image_key.clone(), ent.owner.clone());
        if let Some(blit) = blit_cache.get(&cache_key) {
            items.push((ent.x, ent.y, blit.clone()));
            continue;
        }
        let new_theater = art
            .as_ref()
            .and_then(|a| a.get(&ent.type_id, "NewTheater"))
            .is_some_and(|v| v.eq_ignore_ascii_case("yes"));
        let candidates = if new_theater {
            vec![
                new_theater_shp_name(&image_key, map.theater),
                format!("{}.shp", image_key.to_ascii_lowercase()),
            ]
        } else {
            vec![
                format!("{}.shp", image_key.to_ascii_lowercase()),
                new_theater_shp_name(&image_key, map.theater),
            ]
        };

        let mut loaded: Option<String> = None;
        for file in &candidates {
            if shp_cache.contains_key(file) {
                loaded = Some(file.clone());
                break;
            }
            let Some(bytes) = source.vfs.read(file) else {
                continue;
            };
            let Ok(shp) = ShpFile::parse(&bytes) else {
                continue;
            };
            shp_cache.insert(file.clone(), shp);
            loaded = Some(file.clone());
            break;
        }
        let Some(file) = loaded else {
            continue;
        };
        let Some(shp) = shp_cache.get(&file) else {
            continue;
        };
        let Some(frame) = shp.frames.first() else {
            continue;
        };
        if frame.frame_width == 0 || frame.frame_height == 0 {
            continue;
        }
        let pal = palette_for_owner(&obj_pal, &ent.owner, color_rules);
        let blit = TileBlit {
            width: u32::from(frame.frame_width),
            height: u32::from(frame.frame_height),
            offset_x: i32::from(frame.frame_x),
            offset_y: i32::from(frame.frame_y),
            rgba: frame.to_rgba(&pal),
        };
        blit_cache.insert(cache_key, blit.clone());
        items.push((ent.x, ent.y, blit));
    }

    paint_cell_sprites(image, &items, |x, y| {
        z_lookup.get(&(x, y)).copied().unwrap_or(0)
    })
}

/// 叠画单位 / 步兵 / 飞行器：优先 SHP，否则 VXL 正交投影。
fn paint_mobile_entities(
    source: &GameAssetSource,
    map: &MapInfo,
    image: &mut ra_map::TerrainImage,
    z_lookup: &HashMap<(u16, u16), u8>,
    color_rules: Option<&(IniDocument, ColorSchemes)>,
) -> usize {
    let mobiles: Vec<_> = map
        .entities
        .iter()
        .filter(|e| {
            matches!(
                e.kind,
                MapEntityKind::Unit | MapEntityKind::Infantry | MapEntityKind::Aircraft
            )
        })
        .collect();
    if mobiles.is_empty() {
        return 0;
    }
    let art = source
        .vfs
        .read("art.ini")
        .and_then(|b| IniDocument::parse(&b).ok());
    let obj_pal = source
        .vfs
        .read("unittem.pal")
        .and_then(|b| Palette::parse(&b).ok())
        .or_else(|| {
            source
                .vfs
                .read(theater_palette(map.theater))
                .and_then(|b| Palette::parse(&b).ok())
        });
    let Some(obj_pal) = obj_pal else {
        return 0;
    };
    let vpl = source
        .vfs
        .read("voxels.vpl")
        .and_then(|b| VplFile::parse(&b).ok());

    let mut shp_cache: HashMap<String, ShpFile> = HashMap::new();
    let mut blit_cache: HashMap<(String, u8, String), TileBlit> = HashMap::new();
    let mut items: Vec<(u16, u16, TileBlit)> = Vec::new();

    for ent in mobiles {
        let image_key = art
            .as_ref()
            .and_then(|a| a.get(&ent.type_id, "Image"))
            .unwrap_or(ent.type_id.as_str())
            .to_ascii_uppercase();
        let frame_hint = ent.facing / 32;
        let cache_key = (image_key.clone(), frame_hint, ent.owner.clone());
        if let Some(blit) = blit_cache.get(&cache_key) {
            items.push((ent.x, ent.y, blit.clone()));
            continue;
        }

        let pal = palette_for_owner(&obj_pal, &ent.owner, color_rules);

        if let Some(blit) = load_mobile_shp(
            source,
            &art,
            &image_key,
            map,
            &pal,
            frame_hint,
            &mut shp_cache,
        ) {
            blit_cache.insert(cache_key, blit.clone());
            items.push((ent.x, ent.y, blit));
            continue;
        }

        let stem = image_key.to_ascii_lowercase();
        if let Some(blit) =
            load_mobile_vxl_layers(source, &stem, &pal, vpl.as_ref(), ent.facing, ent.facing)
        {
            blit_cache.insert(cache_key, blit.clone());
            items.push((ent.x, ent.y, blit));
        }
    }

    paint_cell_sprites(image, &items, |x, y| {
        z_lookup.get(&(x, y)).copied().unwrap_or(0)
    })
}

fn load_mobile_vxl_layers(
    source: &GameAssetSource,
    stem: &str,
    pal: &Palette,
    vpl: Option<&VplFile>,
    body_facing: u8,
    turret_facing: u8,
) -> Option<TileBlit> {
    let body_name = format!("{stem}.vxl");
    let body_bytes = source.vfs.read(&body_name)?;
    let body = VxlFile::parse(&body_bytes).ok()?;
    let body_hva = source
        .vfs
        .read(&format!("{stem}.hva"))
        .and_then(|b| HvaFile::parse(&b).ok());

    let mut owned: Vec<(VxlFile, Option<HvaFile>, bool)> = vec![(body, body_hva, false)];
    for suffix in ["tur", "barl", "barrel"] {
        let vxl_name = format!("{stem}{suffix}.vxl");
        let Some(bytes) = source.vfs.read(&vxl_name) else {
            continue;
        };
        let Ok(vxl) = VxlFile::parse(&bytes) else {
            continue;
        };
        let hva = source
            .vfs
            .read(&format!("{stem}{suffix}.hva"))
            .and_then(|b| HvaFile::parse(&b).ok());
        owned.push((vxl, hva, true));
        // `barl` 与 `barrel` 只取先命中的一个。
        if suffix.starts_with("bar") {
            break;
        }
    }

    let layers: Vec<VxlLayerPose<'_>> = owned
        .iter()
        .map(|(v, h, is_turret)| VxlLayerPose {
            vxl: v,
            hva: h.as_ref(),
            facing: if *is_turret {
                turret_facing
            } else {
                body_facing
            },
            frame: 0,
        })
        .collect();
    let sprite = rasterize_vxl_layer_poses(&layers, pal, vpl)?;
    Some(TileBlit {
        width: sprite.width,
        height: sprite.height,
        offset_x: sprite.offset_x + TILE_WIDTH / 2,
        offset_y: sprite.offset_y + TILE_HEIGHT / 2,
        rgba: sprite.rgba,
    })
}

fn load_mobile_shp(
    source: &GameAssetSource,
    art: &Option<IniDocument>,
    image_key: &str,
    map: &MapInfo,
    obj_pal: &Palette,
    frame_hint: u8,
    shp_cache: &mut HashMap<String, ShpFile>,
) -> Option<TileBlit> {
    let new_theater = art
        .as_ref()
        .and_then(|a| a.get(image_key, "NewTheater"))
        .is_some_and(|v| v.eq_ignore_ascii_case("yes"));
    let candidates = if new_theater {
        vec![
            new_theater_shp_name(image_key, map.theater),
            format!("{}.shp", image_key.to_ascii_lowercase()),
        ]
    } else {
        vec![
            format!("{}.shp", image_key.to_ascii_lowercase()),
            new_theater_shp_name(image_key, map.theater),
        ]
    };

    let mut loaded: Option<String> = None;
    for file in &candidates {
        if shp_cache.contains_key(file) {
            loaded = Some(file.clone());
            break;
        }
        let Some(bytes) = source.vfs.read(file) else {
            continue;
        };
        let Ok(shp) = ShpFile::parse(&bytes) else {
            continue;
        };
        shp_cache.insert(file.clone(), shp);
        loaded = Some(file.clone());
        break;
    }
    let file = loaded?;
    let shp = shp_cache.get(&file)?;
    let frame = shp
        .frames
        .get(usize::from(frame_hint))
        .or_else(|| shp.frames.first())?;
    if frame.frame_width == 0 || frame.frame_height == 0 {
        return None;
    }
    Some(TileBlit {
        width: u32::from(frame.frame_width),
        height: u32::from(frame.frame_height),
        offset_x: i32::from(frame.frame_x),
        offset_y: i32::from(frame.frame_y),
        rgba: frame.to_rgba(obj_pal),
    })
}

fn load_preview_terrain(source: &GameAssetSource, theater: Theater) -> Option<(String, RgbaImage)> {
    let pal_bytes = source.vfs.read(theater_palette(theater))?;
    let pal = Palette::parse(&pal_bytes).ok()?;

    let mut candidates: Vec<String> = Vec::new();
    if let Some(ini_bytes) = source.vfs.read(theater_ini_name(theater)) {
        if let Ok(lookup) = parse_tileset_ini(&ini_bytes, theater_tmp_extension(theater)) {
            if let Some(name) = lookup.filename(0) {
                candidates.push(name.to_string());
            }
            for id in [14i32, 9, 10, 12] {
                if let Some(name) = lookup.filename(id) {
                    candidates.push(name.to_string());
                }
            }
        }
    }
    candidates.push(format!("clear01.{}", theater_tmp_extension(theater)));

    for name in candidates {
        let Some(data) = source.vfs.read(&name) else {
            continue;
        };
        let Ok(tmp) = TmpFile::parse(&data) else {
            continue;
        };
        let Some((index, tile)) = tmp
            .tiles
            .iter()
            .enumerate()
            .find_map(|(i, t)| t.as_ref().map(|tile| (i, tile)))
        else {
            continue;
        };
        let Ok(rgba) = tmp.tile_to_rgba(index, &pal) else {
            continue;
        };
        let image = RgbaImage::new(tile.pixel_width, tile.pixel_height, rgba)?;
        return Some((format!("{name}#{index}"), image));
    }
    None
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

fn load_boot_map(
    source: &mut GameAssetSource,
    edition: GameEdition,
    note: &mut String,
) -> MapInfo {
    for name in BOOT_MAP_CANDIDATES {
        let Some(bytes) = source.vfs.read(name) else {
            continue;
        };
        match try_parse_boot_map(edition, name, &bytes) {
            Ok(map) => {
                let theater_mounted = mount_theater_mixes(map.theater, &mut |mix| {
                    matches!(source.vfs.mount_nested(mix), Ok(true))
                });
                *note = format!(
                    "{note} · map:{name} {}x{} {} · 剧院mix {}",
                    map.width,
                    map.height,
                    map.theater.as_str(),
                    theater_mounted
                );
                return map;
            }
            Err(e) => {
                *note = format!("{note} · map:{name} 解析失败（{e}）");
            }
        }
    }
    *note = format!("{note} · map:无");
    MapInfo::empty(edition, "boot")
}

fn boot_world(cfg: &DesktopConfig) -> RaResult<BootResult> {
    let root = cfg.ra2_dir.clone();
    let explicit = match cfg.edition.as_deref() {
        Some(s) => Some(GameEdition::parse(s)?),
        None => None,
    };
    let manifest = detect_edition(&root, explicit)?;
    for report in &manifest.stack.unsupported {
        ra_logger::warn(format!("适配能力缺口 [{}] {}", report.code, report.message));
    }
    if !manifest.stack.extensions.is_empty() {
        let ids: Vec<_> = manifest
            .stack
            .extensions
            .iter()
            .map(|e| e.as_str())
            .collect();
        ra_logger::info(format!("适配扩展探测: {}", ids.join("+")));
    }
    let chain = &manifest.chain;

    let mut source = GameAssetSource::new(manifest.root.clone());
    let (mounted_root, skipped_root) = source.mount_present_roots(&manifest.present_mixes);
    let mounted_nested = source.mount_nested_names(chain.nested_mix_files);

    let mut note = format!(
        "{} · 根mix {} · 嵌套 {} · 跳过 {} · 缺盘 {}",
        chain.edition.as_str(),
        mounted_root,
        mounted_nested,
        skipped_root,
        manifest.missing_mixes.len()
    );

    let map = load_boot_map(&mut source, chain.edition, &mut note);
    if !map.cells.is_empty() {
        note = format!("{note} · iso#{}", map.cells.len());
    }
    if !map.overlays.is_empty() {
        note = format!("{note} · overlay#{}", map.overlays.len());
    }
    if !map.terrain_objects.is_empty() {
        note = format!("{note} · terrain#{}", map.terrain_objects.len());
    }
    if !map.entities.is_empty() {
        note = format!("{note} · entities#{}", map.entities.len());
    }
    if !map.waypoints.is_empty() {
        note = format!("{note} · wp#{}", map.waypoints.len());
    }

    let mut preview_origin = (0i32, 0i32);
    let preview = match load_map_terrain_preview(&source, &map, chain) {
        Some((name, image, ox, oy)) => {
            note = format!("{note} · preview:{name}");
            preview_origin = (ox, oy);
            Some(image)
        }
        None => match load_preview_terrain(&source, map.theater)
            .or_else(|| load_preview_sprite(&source))
        {
            Some((name, image)) => {
                note = format!("{note} · preview:{name}");
                Some(image)
            }
            None => {
                note = format!("{note} · preview:无");
                None
            }
        },
    };

    let session = match load_rules_chain(&source, chain) {
        Ok(rules) => {
            let sections = rules.rules.sections.len();
            let overlays = rules.overlay_types.len();
            note = format!("{note} · rules#{sections} · overlay_types#{overlays}");
            let techno_n = rules.techno_types.len();
            note = format!("{note} · techno_types#{techno_n}");
            let mut world = World::new(chain.edition, &rules, map);
            let land_sealed = seal_pass_grid_from_tmp(&source, &world.map, &mut world.pass_grid);
            if land_sealed > 0 {
                world.repath_mobiles();
            }
            note = format!(
                "{note} · world_entities#{} bound#{} blocked#{} land#{}",
                world.entities.len(),
                world.bound_techno_count(),
                world.pass_grid.blocked_count(),
                land_sealed
            );
            let rules_bytes = AssetSource::read(&source, chain.rules_ini).unwrap_or_default();
            let fp = Session::build_skirmish_fingerprint(
                chain.edition.as_str(),
                &world.map.name,
                &rules_bytes,
                world.map.width,
                world.map.height,
                world.entities.len(),
            );
            ra_logger::info(format!(
                "fingerprint edition={} map={} rules_hash={:#x}",
                fp.edition, fp.map, fp.rules_hash
            ));
            Some(Session::open_skirmish(
                world,
                note.clone(),
                preview_origin,
                fp,
            ))
        }
        Err(e) => {
            note = format!("{note} · 规则待加载（{e}）");
            None
        }
    };

    Ok(BootResult {
        note,
        session,
        preview,
    })
}

fn main() {
    if let Err(e) = run() {
        // init 可能失败，仍尽量打 stderr。
        eprintln!("ra2 错误: {e}");
        ra_logger::error(format!("致命错误: {e}"));
        std::process::exit(1);
    }
}

fn run() -> RaResult<()> {
    let log_path = ra_logger::init_default(true)?;
    ra_logger::info(format!("ra2 启动 · log={}", log_path.display()));

    let (cfg, cfg_diags) = load_desktop_config_with_diagnostics();
    for d in &cfg_diags {
        ra_logger::warn(format!("配置诊断 {} · {}", d.source, d.message));
    }
    match (&cfg.net_url, &cfg.net_room) {
        (Some(url), room) => ra_logger::info(format!(
            "联机配置预留 url={} room={}（协议未定点，不接 socket）",
            url,
            room.as_deref().unwrap_or("—")
        )),
        (None, _) => ra_logger::info("联机配置：未设 net_url"),
    }
    let boot = match boot_world(&cfg) {
        Ok(v) => v,
        Err(e) => {
            ra_logger::error(format!("启动失败: {e}"));
            BootResult {
                note: format!("启动失败: {e}"),
                session: None,
                preview: None,
            }
        }
    };
    ra_logger::info(format!(
        "boot: {} · session={}",
        boot.note,
        if boot.session.is_some() { "ok" } else { "none" }
    ));
    if let Some(session) = boot.session.as_ref() {
        ra_logger::info(format!(
            "preview_origin=({}, {}) entities={}",
            session.preview_origin_x,
            session.preview_origin_y,
            session.world.entities.len()
        ));
    }

    let event_loop = EventLoop::new().map_err(|e| RaError::Msg(e.to_string()))?;
    event_loop.set_control_flow(ControlFlow::Poll);

    let mut app = App::new(boot.note, boot.session, boot.preview);
    event_loop
        .run_app(&mut app)
        .map_err(|e| RaError::Msg(e.to_string()))?;
    ra_logger::info("事件循环结束");
    Ok(())
}
