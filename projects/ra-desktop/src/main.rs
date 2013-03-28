//! 原生 GUI 入口：二进制名 `ra2`（Windows 上为 `ra2.exe`）。
//!
//! 不是命令行工具——启动配置来自 exe/工作目录旁的 `config.toml`，然后由窗口接管进程。

#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod config;
mod fs_source;

use std::collections::HashMap;
use std::sync::Arc;

use ra_adaptor::{detect_edition, find_ci_file};
use ra_assets::{
    rasterize_vxl_posed, HvaFile, IniDocument, Palette, ShpFile, TmpFile, VxlFile,
};
use ra_map::{
    compose_terrain_rgba, new_theater_shp_name, paint_cell_sprites, paint_overlay_markers,
    parse_tileset_ini, theater_ini_name, theater_mix_names, theater_palette,
    theater_tmp_extension, MapEntityKind, MapInfo, Theater, TileBlit, TILE_HEIGHT, TILE_WIDTH,
};
use ra_renderer::{Renderer, RgbaImage};
use ra_rules::{load_rules, OverlayTypeRegistry};
use ra_types::{GameEdition, RaError, RaResult};
use ra_world::World;
use winit::application::ApplicationHandler;
use winit::event::{ElementState, MouseButton, MouseScrollDelta, WindowEvent};
use winit::event_loop::{ActiveEventLoop, ControlFlow, EventLoop};
use winit::keyboard::{KeyCode, PhysicalKey};
use winit::window::{Window, WindowId};

use crate::config::DesktopConfig;
use crate::fs_source::GameAssetSource;

struct App {
    window: Option<Arc<Window>>,
    title_base: String,
    boot_note: String,
    world: Option<World>,
    renderer: Renderer,
    /// 左键拖拽中：上一帧光标位置。
    drag_last: Option<(f64, f64)>,
    /// 已按下左键，等待第一次 CursorMoved 建立起点。
    drag_armed: bool,
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
            drag_last: None,
            drag_armed: false,
        }
    }

    fn refresh_title(&self) {
        if let Some(window) = &self.window {
            let tick = self.world.as_ref().map(|w| w.tick).unwrap_or(0);
            let zoom = self.renderer.camera().zoom;
            window.set_title(&format!(
                "{} · {} · t{} · z{:.2}",
                self.title_base, self.boot_note, tick, zoom
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
                "ra2 gpu: {} · preview={} · zoom={:.2}",
                self.renderer.backend_name(),
                if self.renderer.has_preview() {
                    "yes"
                } else {
                    "no"
                },
                self.renderer.camera().zoom
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
            WindowEvent::MouseInput {
                state,
                button: MouseButton::Left,
                ..
            } => match state {
                ElementState::Pressed => {
                    self.drag_armed = true;
                    self.drag_last = None;
                }
                ElementState::Released => {
                    self.drag_armed = false;
                    self.drag_last = None;
                }
            },
            WindowEvent::CursorMoved { position, .. } => {
                if self.drag_armed {
                    if let Some((lx, ly)) = self.drag_last {
                        let dx = (position.x - lx) as f32;
                        let dy = (position.y - ly) as f32;
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
                    _ => {}
                }
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

fn load_map_terrain_preview(
    source: &GameAssetSource,
    map: &MapInfo,
) -> Option<(String, RgbaImage)> {
    if map.cells.is_empty() {
        return None;
    }
    let pal_bytes = source.vfs.read(theater_palette(map.theater))?;
    let pal = Palette::parse(&pal_bytes).ok()?;
    let ini_bytes = source.vfs.read(theater_ini_name(map.theater))?;
    let lookup =
        parse_tileset_ini(&ini_bytes, theater_tmp_extension(map.theater)).ok()?;

    let mut file_cache: HashMap<String, TmpFile> = HashMap::new();
    let mut blit_cache: HashMap<(i32, u8), TileBlit> = HashMap::new();

    let mut resolve = |tile_num: i32, sub_tile: u8| -> Option<TileBlit> {
        if let Some(blit) = blit_cache.get(&(tile_num, sub_tile)) {
            return Some(blit.clone());
        }
        let name = lookup.filename(tile_num)?.to_string();
        if !file_cache.contains_key(&name) {
            let data = source.vfs.read(&name)?;
            let tmp = TmpFile::parse(&data).ok()?;
            file_cache.insert(name.clone(), tmp);
        }
        let tmp = file_cache.get(&name)?;
        let index = usize::from(sub_tile);
        let tile = tmp.tiles.get(index)?.as_ref()?;
        let rgba = tmp.tile_to_rgba(index, &pal).ok()?;
        let blit = TileBlit {
            width: tile.pixel_width,
            height: tile.pixel_height,
            offset_x: tile.offset_x,
            offset_y: tile.offset_y,
            rgba,
        };
        blit_cache.insert((tile_num, sub_tile), blit.clone());
        Some(blit)
    };

    let mut image = compose_terrain_rgba(&map.cells, &mut resolve)?;
    let mut z_lookup: HashMap<(u16, u16), u8> = HashMap::new();
    for cell in &map.cells {
        z_lookup.insert((cell.x as u16, cell.y as u16), cell.z);
    }
    let (overlay_shp, overlay_mark) = paint_overlays(source, map, &mut image, &z_lookup);
    let terrain_painted = paint_terrain_objects(source, map, &mut image, &z_lookup);
    let structure_painted = paint_structure_entities(source, map, &mut image, &z_lookup);
    let mobile_painted = paint_mobile_entities(source, map, &mut image, &z_lookup);
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
    ))
}

/// 优先叠 Overlay SHP；解析失败的格子回退色块标记。
fn paint_overlays(
    source: &GameAssetSource,
    map: &MapInfo,
    image: &mut ra_map::TerrainImage,
    z_lookup: &HashMap<(u16, u16), u8>,
) -> (usize, usize) {
    if map.overlays.is_empty() {
        return (0, 0);
    }
    let registry = source
        .vfs
        .read("rules.ini")
        .and_then(|b| IniDocument::parse(&b).ok())
        .map(|doc| OverlayTypeRegistry::from_rules(&doc))
        .unwrap_or_default();
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
        let mark = paint_overlay_markers(image, &map.overlays, |x, y| {
            z_lookup.get(&(x, y)).copied().unwrap_or(0)
        });
        return (0, mark);
    };

    let ext = theater_tmp_extension(map.theater);
    let mut shp_cache: HashMap<String, ShpFile> = HashMap::new();
    // (type_name, frame) → blit
    let mut blit_cache: HashMap<(String, u8), TileBlit> = HashMap::new();
    let mut items: Vec<(u16, u16, TileBlit)> = Vec::new();
    let mut unresolved = Vec::new();

    for cell in &map.overlays {
        let Some(type_name) = registry.name(cell.overlay_id).map(str::to_owned) else {
            unresolved.push(*cell);
            continue;
        };
        let image_key = art
            .as_ref()
            .and_then(|a| a.get(&type_name, "Image"))
            .unwrap_or(type_name.as_str())
            .to_ascii_uppercase();
        let frame_idx = cell.data;
        let cache_key = (image_key.clone(), frame_idx);
        if let Some(blit) = blit_cache.get(&cache_key) {
            items.push((cell.x, cell.y, blit.clone()));
            continue;
        }

        let new_theater = art
            .as_ref()
            .and_then(|a| a.get(&type_name, "NewTheater"))
            .is_some_and(|v| v.eq_ignore_ascii_case("yes"));
        let theater_yes = art
            .as_ref()
            .and_then(|a| a.get(&type_name, "Theater"))
            .is_some_and(|v| v.eq_ignore_ascii_case("yes"));
        let mut candidates = Vec::new();
        if theater_yes {
            candidates.push(format!("{}.{ext}", image_key.to_ascii_lowercase()));
        }
        if new_theater {
            candidates.push(new_theater_shp_name(&image_key, map.theater));
        }
        candidates.push(format!("{}.shp", image_key.to_ascii_lowercase()));
        candidates.push(new_theater_shp_name(&image_key, map.theater));
        candidates.push(format!("{}.{ext}", image_key.to_ascii_lowercase()));

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
            unresolved.push(*cell);
            continue;
        };
        let Some(shp) = shp_cache.get(&file) else {
            unresolved.push(*cell);
            continue;
        };
        let frame = shp
            .frames
            .get(usize::from(frame_idx))
            .or_else(|| shp.frames.first());
        let Some(frame) = frame else {
            unresolved.push(*cell);
            continue;
        };
        if frame.frame_width == 0 || frame.frame_height == 0 {
            unresolved.push(*cell);
            continue;
        }
        let blit = TileBlit {
            width: u32::from(frame.frame_width),
            height: u32::from(frame.frame_height),
            offset_x: i32::from(frame.frame_x),
            offset_y: i32::from(frame.frame_y),
            rgba: frame.to_rgba(&obj_pal),
        };
        blit_cache.insert(cache_key, blit.clone());
        items.push((cell.x, cell.y, blit));
    }

    let shp_n = paint_cell_sprites(image, &items, |x, y| {
        z_lookup.get(&(x, y)).copied().unwrap_or(0)
    });
    let mark_n = if unresolved.is_empty() {
        0
    } else {
        paint_overlay_markers(image, &unresolved, |x, y| {
            z_lookup.get(&(x, y)).copied().unwrap_or(0)
        })
    };
    (shp_n, mark_n)
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
fn paint_structure_entities(
    source: &GameAssetSource,
    map: &MapInfo,
    image: &mut ra_map::TerrainImage,
    z_lookup: &HashMap<(u16, u16), u8>,
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
        let pal = obj_pal.for_owner(&ent.owner);
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

        let pal = obj_pal.for_owner(&ent.owner);

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
        let vxl_file = format!("{stem}.vxl");
        let hva_file = format!("{stem}.hva");
        if let Some(bytes) = source.vfs.read(&vxl_file) {
            if let Ok(vxl) = VxlFile::parse(&bytes) {
                let hva = source
                    .vfs
                    .read(&hva_file)
                    .and_then(|b| HvaFile::parse(&b).ok());
                if let Some(sprite) =
                    rasterize_vxl_posed(&vxl, &pal, hva.as_ref(), ent.facing)
                {
                    let blit = TileBlit {
                        width: sprite.width,
                        height: sprite.height,
                        offset_x: sprite.offset_x + TILE_WIDTH / 2,
                        offset_y: sprite.offset_y + TILE_HEIGHT / 2,
                        rgba: sprite.rgba,
                    };
                    blit_cache.insert(cache_key, blit.clone());
                    items.push((ent.x, ent.y, blit));
                }
            }
        }
    }

    paint_cell_sprites(image, &items, |x, y| {
        z_lookup.get(&(x, y)).copied().unwrap_or(0)
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
    const CANDIDATES: &[&str] = &["mp03t4.map", "mp01t4.map", "mp01t2.map", "mp02t4.map"];
    for name in CANDIDATES {
        let Some(bytes) = source.vfs.read(name) else {
            continue;
        };
        match MapInfo::parse_ini(edition, *name, &bytes) {
            Ok(map) => {
                let mut theater_mounted = 0usize;
                for mix_name in theater_mix_names(map.theater) {
                    match source.vfs.mount_nested(mix_name) {
                        Ok(true) => theater_mounted += 1,
                        Ok(false) => {}
                        Err(_) => {}
                    }
                }
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

    let preview = match load_map_terrain_preview(&source, &map)
        .or_else(|| load_preview_terrain(&source, map.theater))
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
    };

    let world = match load_rules(&source, chain.edition) {
        Ok(rules) => {
            let sections = rules.rules.sections.len();
            let overlays = rules.overlay_types.len();
            note = format!("{note} · rules#{sections} · overlay_types#{overlays}");
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
