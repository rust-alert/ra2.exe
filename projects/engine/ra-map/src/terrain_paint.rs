//! 地图 `[Terrain]` 物件 SHP 叠画。

use std::collections::HashMap;

use ra_assets::{Palette, ShpFile, shp_body_frame_count, shp_shadow_half_base, shp_shadow_half_populated};
use ra_types::AssetSource;

use crate::{
    LightingConfig, MapInfo, PointLight,
    compose::{ShadowBlit, TerrainImage, TileBlit, paint_cell_sprites},
    iso_math::{TILE_HEIGHT, TILE_WIDTH},
    lighting::{apply_rgba_tint, cell_tint_with_lights},
    theater::{theater_palette, theater_tmp_extension},
};

/// FA2 `IsoView` 对普通地形物件（树/岩）的额外 Y（钻石中心叠画后再偏 −3）。
const TERRAIN_OBJECT_Y_FUDGE: i32 = -3;

/// `SpawnsTiberium=yes` 矿柱相对格子钻石中心的 `CellHeight` Y 偏移（−15）。
///
/// 呈现时再叠加 FA2 地形 −3，与矿石 overlay 的 −12 偏置对齐后，矿柱相对矿田约再高 6px。
const SPAWNS_TIBERIUM_Y_FUDGE: i32 = -15;

/// 逻辑帧率：`rules` 的 `AnimationRate` 以该帧率为单位间隔。
const TERRAIN_LOGIC_FPS: u32 = 15;

/// 地形物件叠画模式。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TerrainPaintMode {
    /// 静态物件。常循环 `IsAnimated` 与 `SpawnsTiberium` 矿柱均留给活动层银行。
    StaticOnly,
    /// 静态与矿柱 Idle 画第 0 帧；仅非产矿的常循环动画按 `anim_clock_ms` 选帧。
    AllWithClock {
        /// 呈现时钟（毫秒）。
        anim_clock_ms: u64,
    },
}

/// 单个动画地形物件的预解码帧序列。
#[derive(Debug, Clone)]
pub struct TerrainAnimLayer {
    /// rules / 地图节名（如 `TIBTRE01`）。
    pub type_name: String,
    /// 格子 X。
    pub x: u16,
    /// 格子 Y。
    pub y: u16,
    /// 格子高度（叠画用）。
    pub cell_z: u8,
    /// `rules` `AnimationRate` 换算后的毫秒/帧。
    pub rate_ms: u32,
    /// 已解码主体帧（下标 0..body_n）。
    pub frames: Vec<TileBlit>,
    /// 命中的剧院 SHP 逻辑名（如 `tibtre01.tem`）。
    pub file: String,
    /// SHP 画布宽。
    pub canvas_width: u16,
    /// SHP 画布高。
    pub canvas_height: u16,
    /// SHP 总帧数（含落影半幅，便于与 CLI `frames=` 对照）。
    pub shp_frames: usize,
    /// 实际用于解码的调色板逻辑名（如 `unittem.pal` / `isotem.pal`）。
    pub palette: String,
    /// SHP 后半是否为落影半幅（资源侧）。
    pub has_shadow_frames: bool,
    /// 主体 `TileBlit` 是否已挂上落影模板（呈现侧）。
    pub shadow_blit_attached: bool,
}

/// 地图上全部动画地形物件（装载时烘焙，对局按时钟选帧）。
#[derive(Debug, Clone)]
pub struct TerrainAnimBank {
    /// 收集时的地图 `[Lighting]`（播帧时按格 tint）。
    pub lighting: LightingConfig,
    /// 收集时的点光源。
    pub point_lights: Vec<PointLight>,
    /// 活动层列表。
    pub layers: Vec<TerrainAnimLayer>,
}

impl Default for TerrainAnimBank {
    fn default() -> Self {
        Self { lighting: LightingConfig::default(), point_lights: Vec::new(), layers: Vec::new() }
    }
}

impl TerrainAnimBank {
    /// 是否有可播动画地形。
    pub fn is_empty(&self) -> bool {
        self.layers.is_empty()
    }

    /// 当前时钟下各层帧号签名（用于跳过无变化的预览刷新）。
    pub fn frame_signature(&self, clock_ms: u64) -> u64 {
        let mut h: u64 = 0xcbf29ce484222325;
        for layer in &self.layers {
            let body_n = layer.frames.len();
            let frame = if body_n == 0 {
                0
            }
            else {
                let rate = u64::from(layer.rate_ms.max(1));
                ((clock_ms / rate) % body_n as u64) as u16
            };
            h ^= u64::from(frame);
            h = h.wrapping_mul(0x100000001b3);
            h ^= u64::from(layer.x) << 16 | u64::from(layer.y);
            h = h.wrapping_mul(0x100000001b3);
        }
        h
    }
}

/// 将 `rules` `AnimationRate`（逻辑帧间隔）换算为毫秒/动画帧。
pub fn terrain_animation_rate_ms(animation_rate: u32) -> u32 {
    let frames = animation_rate.max(1);
    (frames * 1000) / TERRAIN_LOGIC_FPS
}

/// 按呈现时钟与 `AnimationRate` 选取地形动画主体帧下标。
///
/// `frame_count` 应为 `shp_body_frame_count`（不含落影半幅）。
pub fn terrain_anim_frame(clock_ms: u64, animation_rate: u32, frame_count: usize) -> usize {
    if frame_count == 0 {
        return 0;
    }
    let rate = u64::from(terrain_animation_rate_ms(animation_rate).max(1));
    ((clock_ms / rate) % frame_count as u64) as usize
}

/// 将地形物件叠到合成图上。返回画上的物件数。
///
/// `rules_ini` 提供 `IsAnimated` / `AnimationRate` / `SpawnsTiberium`。
/// `SpawnsTiberium` 矿柱由产矿状态机选帧，经 `collect_ore_tree_anim_bank` 叠画。
/// `StaticOnly` 跳过常循环动画与矿柱；`AllWithClock` 仅对非产矿的 `IsAnimated` 按时钟选帧，矿柱仍画 Idle 第 0 帧。
pub fn paint_map_terrain_objects(
    source: &dyn AssetSource,
    map: &MapInfo,
    image: &mut TerrainImage,
    art_ini: &str,
    rules_ini: &str,
    mode: TerrainPaintMode,
) -> usize {
    if map.terrain_objects.is_empty() {
        return 0;
    }

    let z_lookup: HashMap<(u16, u16), u8> =
        map.cells.iter().filter(|c| c.x >= 0 && c.y >= 0).map(|c| ((c.x as u16, c.y as u16), c.z)).collect();
    let z_at = |x: u16, y: u16| z_lookup.get(&(x, y)).copied().unwrap_or(0);

    let docs = crate::PaintIniDocs::load(source, art_ini, rules_ini);
    let art = docs.art;
    let rules = docs.rules;
    let theater_pal_name = theater_palette(map.theater);
    let theater_pal = source.read(theater_pal_name).ok().and_then(|b| Palette::parse(&b).ok());
    let unit_pal = source.read("unittem.pal").ok().and_then(|b| Palette::parse(&b).ok());
    if theater_pal.is_none() && unit_pal.is_none() {
        return 0;
    }

    let ext = theater_tmp_extension(map.theater);
    let mut shp_cache: HashMap<String, ShpFile> = HashMap::new();
    // (image_key, frame_idx, spawns_tiberium)
    let mut blit_cache: HashMap<(String, u16, bool), TileBlit> = HashMap::new();
    let mut items: Vec<(u16, u16, TileBlit)> = Vec::new();

    for obj in &map.terrain_objects {
        let image_key = art.as_ref().and_then(|a| a.get(&obj.name, "Image")).unwrap_or(obj.name.as_str()).to_ascii_uppercase();
        let animated = rules.as_ref().is_some_and(|r| is_yes(r.get(&obj.name, "IsAnimated")));
        let spawns_tiberium = rules.as_ref().is_some_and(|r| is_yes(r.get(&obj.name, "SpawnsTiberium")));
        // 矿柱：`StaticOnly` 底图不画（由 `OreTree` 银行按状态机帧叠画）；`AllWithClock` 仍可画 Idle 0 供测试。
        let loops_with_clock = animated && !spawns_tiberium;
        let anim_clock_ms = match mode {
            TerrainPaintMode::StaticOnly if loops_with_clock || spawns_tiberium => continue,
            TerrainPaintMode::StaticOnly => 0,
            TerrainPaintMode::AllWithClock { anim_clock_ms } => anim_clock_ms,
        };
        let anim_rate = rules.as_ref().and_then(|r| r.get(&obj.name, "AnimationRate")).and_then(parse_u32).unwrap_or(1);
        let Some(obj_pal) = pick_terrain_palette(spawns_tiberium, theater_pal.as_ref(), unit_pal.as_ref())
        else {
            continue;
        };
        let tint = map.tint_at(obj.x, obj.y, z_at(obj.x, obj.y));

        let file = format!("{}.{ext}", image_key.to_ascii_lowercase());
        if !shp_cache.contains_key(&file) {
            let Ok(bytes) = source.read(&file)
            else {
                continue;
            };
            let Ok(shp) = ShpFile::parse(&bytes)
            else {
                continue;
            };
            shp_cache.insert(file.clone(), shp);
        }
        let Some(shp) = shp_cache.get(&file)
        else {
            continue;
        };
        let body_n = shp_body_frame_count(&shp.frames);
        if body_n == 0 {
            continue;
        }
        let frame_idx = if loops_with_clock { terrain_anim_frame(anim_clock_ms, anim_rate, body_n) as u16 } else { 0 };
        let cache_key = (image_key.clone(), frame_idx, spawns_tiberium);
        if let Some(blit) = blit_cache.get(&cache_key) {
            let mut painted = blit.clone();
            apply_rgba_tint(&mut painted.rgba, tint);
            items.push((obj.x, obj.y, painted));
            continue;
        }
        let Some(frame) = shp.frames.get(usize::from(frame_idx))
        else {
            continue;
        };
        if frame.frame_width == 0 || frame.frame_height == 0 {
            continue;
        }
        let shadow = shadow_blit_for_body(shp, usize::from(frame_idx), spawns_tiberium);
        let mut blit = if spawns_tiberium {
            frame_to_spawns_tiberium_blit(frame, shp.width, shp.height, obj_pal, shadow)
        }
        else {
            frame_to_blit(frame, shp.width, shp.height, obj_pal, shadow)
        };
        blit_cache.insert(cache_key, blit.clone());
        apply_rgba_tint(&mut blit.rgba, tint);
        items.push((obj.x, obj.y, blit));
    }

    paint_cell_sprites(image, &items, z_at)
}

/// 收集常循环的 `IsAnimated` 地形物件并预解码主体帧。
///
/// `SpawnsTiberium` 矿柱不进银行：零售 `AnimationProbability`（如 `.003`）由产矿状态机
/// 触发一次性播到中点帧，平时固定 Idle 第 0 帧，不得用呈现时钟常循环。
pub fn collect_terrain_anim_bank(source: &dyn AssetSource, map: &MapInfo, art_ini: &str, rules_ini: &str) -> TerrainAnimBank {
    if map.terrain_objects.is_empty() {
        return TerrainAnimBank::default();
    }

    let z_lookup: HashMap<(u16, u16), u8> =
        map.cells.iter().filter(|c| c.x >= 0 && c.y >= 0).map(|c| ((c.x as u16, c.y as u16), c.z)).collect();

    let docs = crate::PaintIniDocs::load(source, art_ini, rules_ini);
    let art = docs.art;
    let Some(rules) = docs.rules
    else {
        return TerrainAnimBank { lighting: map.lighting.clone(), point_lights: map.point_lights.clone(), layers: Vec::new() };
    };
    let theater_pal_name = theater_palette(map.theater);
    let theater_pal = source.read(theater_pal_name).ok().and_then(|b| Palette::parse(&b).ok());
    let unit_pal = source.read("unittem.pal").ok().and_then(|b| Palette::parse(&b).ok());
    if theater_pal.is_none() && unit_pal.is_none() {
        return TerrainAnimBank { lighting: map.lighting.clone(), point_lights: map.point_lights.clone(), layers: Vec::new() };
    }

    let ext = theater_tmp_extension(map.theater);
    let mut shp_cache: HashMap<String, ShpFile> = HashMap::new();
    let mut layers = Vec::new();

    for obj in &map.terrain_objects {
        if !is_yes(rules.get(&obj.name, "IsAnimated")) {
            continue;
        }
        // 产矿矿柱：条件动画，不进入呈现时钟循环。
        if is_yes(rules.get(&obj.name, "SpawnsTiberium")) {
            continue;
        }
        let spawns_tiberium = false;
        let Some(obj_pal) = pick_terrain_palette(spawns_tiberium, theater_pal.as_ref(), unit_pal.as_ref())
        else {
            continue;
        };
        let palette_name = theater_pal_name.to_string();
        let anim_rate = rules.get(&obj.name, "AnimationRate").and_then(parse_u32).unwrap_or(1);
        let image_key = art.as_ref().and_then(|a| a.get(&obj.name, "Image")).unwrap_or(obj.name.as_str()).to_ascii_uppercase();
        let file = format!("{}.{ext}", image_key.to_ascii_lowercase());
        if !shp_cache.contains_key(&file) {
            let Ok(bytes) = source.read(&file)
            else {
                continue;
            };
            let Ok(shp) = ShpFile::parse(&bytes)
            else {
                continue;
            };
            shp_cache.insert(file.clone(), shp);
        }
        let Some(shp) = shp_cache.get(&file)
        else {
            continue;
        };
        let body_n = shp_body_frame_count(&shp.frames);
        if body_n == 0 {
            continue;
        }
        let mut frames = Vec::with_capacity(body_n);
        for idx in 0..body_n {
            let Some(frame) = shp.frames.get(idx)
            else {
                break;
            };
            if frame.frame_width == 0 || frame.frame_height == 0 {
                frames.push(TileBlit { width: 0, height: 0, offset_x: 0, offset_y: 0, rgba: Vec::new(), shadow: None });
                continue;
            }
            let shadow = shadow_blit_for_body(shp, idx, false);
            frames.push(frame_to_blit(frame, shp.width, shp.height, obj_pal, shadow));
        }
        if frames.iter().all(|f| f.width == 0) {
            continue;
        }
        let cell_z = z_lookup.get(&(obj.x, obj.y)).copied().unwrap_or(0);
        let has_shadow_frames = shp_shadow_half_populated(&shp.frames);
        let shadow_blit_attached = frames.iter().any(|f| f.shadow.is_some());
        layers.push(TerrainAnimLayer {
            type_name: obj.name.clone(),
            x: obj.x,
            y: obj.y,
            cell_z,
            rate_ms: terrain_animation_rate_ms(anim_rate),
            frames,
            file,
            canvas_width: shp.width,
            canvas_height: shp.height,
            shp_frames: shp.frames.len(),
            palette: palette_name,
            has_shadow_frames,
            shadow_blit_attached,
        });
    }

    TerrainAnimBank { lighting: map.lighting.clone(), point_lights: map.point_lights.clone(), layers }
}

/// 按时钟把动画地形叠到地形图上。
pub fn paint_terrain_anim_bank(image: &mut TerrainImage, bank: &TerrainAnimBank, clock_ms: u64) -> usize {
    if bank.layers.is_empty() {
        return 0;
    }
    let mut items: Vec<(u16, u16, TileBlit)> = Vec::with_capacity(bank.layers.len());
    for layer in &bank.layers {
        let body_n = layer.frames.len();
        if body_n == 0 {
            continue;
        }
        let rate = u64::from(layer.rate_ms.max(1));
        let local = ((clock_ms / rate) % body_n as u64) as usize;
        let Some(blit) = layer.frames.get(local)
        else {
            continue;
        };
        if blit.width == 0 || blit.height == 0 {
            continue;
        }
        let mut painted = blit.clone();
        apply_rgba_tint(&mut painted.rgba, cell_tint_with_lights(&bank.lighting, layer.cell_z, layer.x, layer.y, &bank.point_lights));
        items.push((layer.x, layer.y, painted));
    }
    let z_at = |x: u16, y: u16| bank.layers.iter().find(|l| l.x == x && l.y == y).map(|l| l.cell_z).unwrap_or(0);
    paint_cell_sprites(image, &items, z_at)
}

/// 在已有 RGBA 预览上叠动画地形（保留原点）。
pub fn paint_terrain_anims_onto_rgba(
    image: &mut image::RgbaImage,
    origin_x: i32,
    origin_y: i32,
    bank: &TerrainAnimBank,
    clock_ms: u64,
) -> usize {
    let mut terrain = TerrainImage { image: std::mem::take(image), drawn: 0, origin_x, origin_y };
    let n = paint_terrain_anim_bank(&mut terrain, bank, clock_ms);
    *image = terrain.image;
    n
}

/// 收集 `SpawnsTiberium` 矿柱并预解码全部主体帧（供产矿状态机选帧）。
pub fn collect_ore_tree_anim_bank(source: &dyn AssetSource, map: &MapInfo, art_ini: &str, rules_ini: &str) -> TerrainAnimBank {
    if map.terrain_objects.is_empty() {
        return TerrainAnimBank::default();
    }

    let z_lookup: HashMap<(u16, u16), u8> =
        map.cells.iter().filter(|c| c.x >= 0 && c.y >= 0).map(|c| ((c.x as u16, c.y as u16), c.z)).collect();

    let docs = crate::PaintIniDocs::load(source, art_ini, rules_ini);
    let art = docs.art;
    let Some(rules) = docs.rules
    else {
        return TerrainAnimBank { lighting: map.lighting.clone(), point_lights: map.point_lights.clone(), layers: Vec::new() };
    };
    let theater_pal_name = theater_palette(map.theater);
    let theater_pal = source.read(theater_pal_name).ok().and_then(|b| Palette::parse(&b).ok());
    let unit_pal = source.read("unittem.pal").ok().and_then(|b| Palette::parse(&b).ok());
    if theater_pal.is_none() && unit_pal.is_none() {
        return TerrainAnimBank { lighting: map.lighting.clone(), point_lights: map.point_lights.clone(), layers: Vec::new() };
    }

    let ext = theater_tmp_extension(map.theater);
    let mut shp_cache: HashMap<String, ShpFile> = HashMap::new();
    let mut layers = Vec::new();

    for obj in &map.terrain_objects {
        if !is_yes(rules.get(&obj.name, "SpawnsTiberium")) {
            continue;
        }
        let Some(obj_pal) = pick_terrain_palette(true, theater_pal.as_ref(), unit_pal.as_ref())
        else {
            continue;
        };
        let anim_rate = rules.get(&obj.name, "AnimationRate").and_then(parse_u32).unwrap_or(1);
        let image_key = art.as_ref().and_then(|a| a.get(&obj.name, "Image")).unwrap_or(obj.name.as_str()).to_ascii_uppercase();
        let file = format!("{}.{ext}", image_key.to_ascii_lowercase());
        if !shp_cache.contains_key(&file) {
            let Ok(bytes) = source.read(&file)
            else {
                continue;
            };
            let Ok(shp) = ShpFile::parse(&bytes)
            else {
                continue;
            };
            shp_cache.insert(file.clone(), shp);
        }
        let Some(shp) = shp_cache.get(&file)
        else {
            continue;
        };
        let body_n = shp_body_frame_count(&shp.frames);
        if body_n == 0 {
            continue;
        }
        let mut frames = Vec::with_capacity(body_n);
        for idx in 0..body_n {
            let Some(frame) = shp.frames.get(idx)
            else {
                break;
            };
            if frame.frame_width == 0 || frame.frame_height == 0 {
                frames.push(TileBlit { width: 0, height: 0, offset_x: 0, offset_y: 0, rgba: Vec::new(), shadow: None });
                continue;
            }
            let shadow = shadow_blit_for_body(shp, idx, true);
            frames.push(frame_to_spawns_tiberium_blit(frame, shp.width, shp.height, obj_pal, shadow));
        }
        if frames.iter().all(|f| f.width == 0) {
            continue;
        }
        let cell_z = z_lookup.get(&(obj.x, obj.y)).copied().unwrap_or(0);
        let has_shadow_frames = shp_shadow_half_populated(&shp.frames);
        let shadow_blit_attached = frames.iter().any(|f| f.shadow.is_some());
        layers.push(TerrainAnimLayer {
            type_name: obj.name.clone(),
            x: obj.x,
            y: obj.y,
            cell_z,
            rate_ms: terrain_animation_rate_ms(anim_rate),
            frames,
            file,
            canvas_width: shp.width,
            canvas_height: shp.height,
            shp_frames: shp.frames.len(),
            palette: "unittem.pal".into(),
            has_shadow_frames,
            shadow_blit_attached,
        });
    }

    TerrainAnimBank { lighting: map.lighting.clone(), point_lights: map.point_lights.clone(), layers }
}

/// 导出 `(x, y, shp_frames)`，供仿真回写矿柱总帧数。
pub fn ore_tree_frame_count_hints(bank: &TerrainAnimBank) -> Vec<(u16, u16, u16)> {
    bank.layers.iter().map(|l| (l.x, l.y, l.shp_frames.min(u16::MAX as usize) as u16)).collect()
}

/// 矿柱 / 动画地形一层的运行时诊断行（装载与对照用）。
pub fn format_terrain_anim_layer_diag(
    layer: &TerrainAnimLayer,
    selected_frame: u16,
    static_layer_drawn: bool,
    anim_bank_drawn: bool,
) -> String {
    format!(
        "type={} file={} palette={} canvas={}x{} shp_frames={} body_frames={} selected_frame={} rate_ms={} static_layer_drawn={} anim_bank_drawn={} has_shadow_frames={} shadow_blit_attached={}",
        layer.type_name,
        layer.file,
        layer.palette,
        layer.canvas_width,
        layer.canvas_height,
        layer.shp_frames,
        layer.frames.len(),
        selected_frame,
        layer.rate_ms,
        static_layer_drawn,
        anim_bank_drawn,
        layer.has_shadow_frames,
        layer.shadow_blit_attached
    )
}

/// 按状态机给出的 `(x, y, frame)` 叠画矿柱。
pub fn paint_ore_tree_frames(image: &mut TerrainImage, bank: &TerrainAnimBank, frames: &[(u16, u16, u16)]) -> usize {
    if bank.layers.is_empty() || frames.is_empty() {
        return 0;
    }
    let mut items: Vec<(u16, u16, TileBlit)> = Vec::with_capacity(frames.len());
    for &(x, y, frame_idx) in frames {
        let Some(layer) = bank.layers.iter().find(|l| l.x == x && l.y == y)
        else {
            continue;
        };
        let body_n = layer.frames.len();
        if body_n == 0 {
            continue;
        }
        let local = usize::from(frame_idx) % body_n;
        let Some(blit) = layer.frames.get(local)
        else {
            continue;
        };
        if blit.width == 0 || blit.height == 0 {
            continue;
        }
        let mut painted = blit.clone();
        apply_rgba_tint(&mut painted.rgba, cell_tint_with_lights(&bank.lighting, layer.cell_z, layer.x, layer.y, &bank.point_lights));
        items.push((x, y, painted));
    }
    let z_at = |x: u16, y: u16| bank.layers.iter().find(|l| l.x == x && l.y == y).map(|l| l.cell_z).unwrap_or(0);
    paint_cell_sprites(image, &items, z_at)
}

/// 在已有 RGBA 预览上按状态机帧叠画矿柱。
pub fn paint_ore_tree_frames_onto_rgba(
    image: &mut image::RgbaImage,
    origin_x: i32,
    origin_y: i32,
    bank: &TerrainAnimBank,
    frames: &[(u16, u16, u16)],
) -> usize {
    let mut terrain = TerrainImage { image: std::mem::take(image), drawn: 0, origin_x, origin_y };
    let n = paint_ore_tree_frames(&mut terrain, bank, frames);
    *image = terrain.image;
    n
}

fn frame_to_blit(frame: &ra_assets::ShpFrame, shp_w: u16, shp_h: u16, pal: &Palette, shadow: Option<ShadowBlit>) -> TileBlit {
    // 普通树/岩：子帧相对整幅画布裁切，锚在钻石中心（再加 FA2 −3 Y）。
    TileBlit {
        width: u32::from(frame.frame_width),
        height: u32::from(frame.frame_height),
        offset_x: i32::from(frame.frame_x as i16) - i32::from(shp_w) / 2 + TILE_WIDTH / 2,
        offset_y: i32::from(frame.frame_y as i16) - i32::from(shp_h) / 2 + TILE_HEIGHT / 2 + TERRAIN_OBJECT_Y_FUDGE,
        rgba: frame.to_rgba(pal),
        shadow,
    }
}

/// `SpawnsTiberium` 矿柱：子帧贴回完整 SHP 画布，相对格子钻石中心锚定。
///
/// Y = −CellHeight(−15) + FA2 地形 fudge(−3)。`paint_cell_sprites` 以 `iso_to_screen`
/// （钻石包围盒原点）为基准，因此偏移为
/// `(TILE_WIDTH/2 − w/2, TILE_HEIGHT/2 − h/2 − 18)`。
fn frame_to_spawns_tiberium_blit(frame: &ra_assets::ShpFrame, shp_w: u16, shp_h: u16, pal: &Palette, shadow: Option<ShadowBlit>) -> TileBlit {
    let full_w = u32::from(shp_w);
    let full_h = u32::from(shp_h);
    let mut rgba = vec![0u8; (full_w * full_h * 4) as usize];
    paste_rgba_to_canvas(&frame.to_rgba(pal), frame, full_w, full_h, &mut rgba);
    TileBlit {
        width: full_w,
        height: full_h,
        offset_x: TILE_WIDTH / 2 - i32::from(shp_w) / 2,
        offset_y: TILE_HEIGHT / 2 - i32::from(shp_h) / 2 + SPAWNS_TIBERIUM_Y_FUDGE + TERRAIN_OBJECT_Y_FUDGE,
        rgba,
        shadow,
    }
}

fn shadow_blit_for_body(shp: &ShpFile, body_idx: usize, spawns_tiberium: bool) -> Option<ShadowBlit> {
    if !shp_shadow_half_populated(&shp.frames) {
        return None;
    }
    let base = shp_shadow_half_base(shp.frames.len())?;
    let frame = shp.frames.get(base + body_idx)?;
    if frame.frame_width == 0 || frame.frame_height == 0 {
        return None;
    }
    if spawns_tiberium {
        Some(frame_to_spawns_tiberium_shadow(frame, shp.width, shp.height))
    }
    else {
        Some(frame_to_cropped_shadow(frame, shp.width, shp.height, TERRAIN_OBJECT_Y_FUDGE))
    }
}

fn frame_to_cropped_shadow(frame: &ra_assets::ShpFrame, shp_w: u16, shp_h: u16, y_fudge: i32) -> ShadowBlit {
    ShadowBlit {
        width: u32::from(frame.frame_width),
        height: u32::from(frame.frame_height),
        offset_x: i32::from(frame.frame_x as i16) - i32::from(shp_w) / 2 + TILE_WIDTH / 2,
        offset_y: i32::from(frame.frame_y as i16) - i32::from(shp_h) / 2 + TILE_HEIGHT / 2 + y_fudge,
        mask: frame.pixels.clone(),
    }
}

fn frame_to_spawns_tiberium_shadow(frame: &ra_assets::ShpFrame, shp_w: u16, shp_h: u16) -> ShadowBlit {
    let full_w = u32::from(shp_w);
    let full_h = u32::from(shp_h);
    let mut mask = vec![0u8; (full_w * full_h) as usize];
    paste_indices_to_canvas(&frame.pixels, frame, full_w, full_h, &mut mask);
    ShadowBlit {
        width: full_w,
        height: full_h,
        offset_x: TILE_WIDTH / 2 - i32::from(shp_w) / 2,
        offset_y: TILE_HEIGHT / 2 - i32::from(shp_h) / 2 + SPAWNS_TIBERIUM_Y_FUDGE + TERRAIN_OBJECT_Y_FUDGE,
        mask,
    }
}

fn paste_rgba_to_canvas(src: &[u8], frame: &ra_assets::ShpFrame, full_w: u32, full_h: u32, rgba: &mut [u8]) {
    let fw = u32::from(frame.frame_width);
    let fh = u32::from(frame.frame_height);
    let fx = u32::from(frame.frame_x);
    let fy = u32::from(frame.frame_y);
    for y in 0..fh {
        let dst_y = fy + y;
        if dst_y >= full_h {
            break;
        }
        let copy_w = fw.min(full_w.saturating_sub(fx));
        let src_off = (y * fw * 4) as usize;
        let dst_off = ((dst_y * full_w + fx) * 4) as usize;
        let bytes = (copy_w * 4) as usize;
        if src_off + bytes <= src.len() && dst_off + bytes <= rgba.len() {
            rgba[dst_off..dst_off + bytes].copy_from_slice(&src[src_off..src_off + bytes]);
        }
    }
}

fn paste_indices_to_canvas(src: &[u8], frame: &ra_assets::ShpFrame, full_w: u32, full_h: u32, dst: &mut [u8]) {
    let fw = u32::from(frame.frame_width);
    let fh = u32::from(frame.frame_height);
    let fx = u32::from(frame.frame_x);
    let fy = u32::from(frame.frame_y);
    for y in 0..fh {
        let dst_y = fy + y;
        if dst_y >= full_h {
            break;
        }
        for x in 0..fw {
            let dst_x = fx + x;
            if dst_x >= full_w {
                break;
            }
            let si = (y * fw + x) as usize;
            let Some(&idx) = src.get(si)
            else {
                break;
            };
            if idx == 0 {
                continue;
            }
            let di = (dst_y * full_w + dst_x) as usize;
            if di < dst.len() {
                dst[di] = 1;
            }
        }
    }
}

fn pick_terrain_palette<'a>(spawns_tiberium: bool, theater_pal: Option<&'a Palette>, unit_pal: Option<&'a Palette>) -> Option<&'a Palette> {
    if spawns_tiberium { unit_pal.or(theater_pal) } else { theater_pal.or(unit_pal) }
}

fn is_yes(raw: Option<&str>) -> bool {
    raw.is_some_and(|v| v.eq_ignore_ascii_case("yes"))
}

fn parse_u32(raw: &str) -> Option<u32> {
    raw.trim().parse().ok()
}
