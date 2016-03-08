//! 地图 `[Terrain]` 物件 SHP 叠画。

use std::collections::HashMap;

use ra_assets::{IniDocument, Palette, ShpFile, shp_body_frame_count};
use ra_types::AssetSource;

use crate::{
    LightingConfig, MapInfo, PointLight,
    compose::{TerrainImage, TileBlit, paint_cell_sprites},
    iso_math::{TILE_HEIGHT, TILE_WIDTH},
    lighting::{apply_rgba_tint, cell_tint_with_lights},
    theater::{theater_palette, theater_tmp_extension},
};

/// FA2 `IsoView` 对地形物件（树/岩）的额外 Y（钻石中心叠画后再偏 −3）。
const TERRAIN_OBJECT_Y_FUDGE: i32 = -3;

/// 逻辑帧率：`rules` 的 `AnimationRate` 以该帧率为单位间隔。
const TERRAIN_LOGIC_FPS: u32 = 15;

/// 地形物件叠画模式。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TerrainPaintMode {
    /// 只画静态物件（`IsAnimated` 以外）；动画物件留给 `TerrainAnimBank`。
    StaticOnly,
    /// 静态画第 0 帧；动画物件按 `anim_clock_ms` 选帧并画上。
    AllWithClock {
        /// 呈现时钟（毫秒）。
        anim_clock_ms: u64,
    },
}

/// 单个动画地形物件的预解码帧序列。
#[derive(Debug, Clone)]
pub struct TerrainAnimLayer {
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
        Self {
            lighting: LightingConfig::default(),
            point_lights: Vec::new(),
            layers: Vec::new(),
        }
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
            } else {
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
/// `rules_ini` 提供 `IsAnimated` / `AnimationRate`。
/// `StaticOnly` 跳过动画物件；`AllWithClock` 按时钟画动画帧。
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

    let art = source.read(art_ini).ok().and_then(|b| IniDocument::parse(&b).ok());
    let rules = source.read(rules_ini).ok().and_then(|b| IniDocument::parse(&b).ok());
    // `Theater=yes` 地形物件统一使用等距剧院调色板。`SpawnsTiberium` 等玩法字段
    // 不改变 SHP 的索引语义，矿柱也属于这一资源族。
    let obj_pal = source
        .read(theater_palette(map.theater))
        .ok()
        .and_then(|b| Palette::parse(&b).ok())
        .or_else(|| source.read("unittem.pal").ok().and_then(|b| Palette::parse(&b).ok()));
    let Some(obj_pal) = obj_pal
    else {
        return 0;
    };

    let ext = theater_tmp_extension(map.theater);
    let mut shp_cache: HashMap<String, ShpFile> = HashMap::new();
    let mut blit_cache: HashMap<(String, u16), TileBlit> = HashMap::new();
    let mut items: Vec<(u16, u16, TileBlit)> = Vec::new();

    for obj in &map.terrain_objects {
        let image_key = art.as_ref().and_then(|a| a.get(&obj.name, "Image")).unwrap_or(obj.name.as_str()).to_ascii_uppercase();
        let animated = rules.as_ref().is_some_and(|r| is_yes(r.get(&obj.name, "IsAnimated")));
        let anim_clock_ms = match mode {
            TerrainPaintMode::StaticOnly if animated => continue,
            TerrainPaintMode::StaticOnly => 0,
            TerrainPaintMode::AllWithClock { anim_clock_ms } => anim_clock_ms,
        };
        let anim_rate = rules
            .as_ref()
            .and_then(|r| r.get(&obj.name, "AnimationRate"))
            .and_then(parse_u32)
            .unwrap_or(1);
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
        let frame_idx = if animated {
            terrain_anim_frame(anim_clock_ms, anim_rate, body_n) as u16
        } else {
            0
        };
        let cache_key = (image_key.clone(), frame_idx);
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
        // 与 overlay / 建筑一致：子帧相对整幅画布裁切，锚在钻石中心（再加 FA2 −3 Y）。
        let mut blit = frame_to_blit(frame, shp.width, shp.height, &obj_pal);
        blit_cache.insert(cache_key, blit.clone());
        apply_rgba_tint(&mut blit.rgba, tint);
        items.push((obj.x, obj.y, blit));
    }

    paint_cell_sprites(image, &items, z_at)
}

/// 收集 `IsAnimated=yes` 地形物件并预解码全部主体帧。
pub fn collect_terrain_anim_bank(
    source: &dyn AssetSource,
    map: &MapInfo,
    art_ini: &str,
    rules_ini: &str,
) -> TerrainAnimBank {
    if map.terrain_objects.is_empty() {
        return TerrainAnimBank::default();
    }

    let z_lookup: HashMap<(u16, u16), u8> =
        map.cells.iter().filter(|c| c.x >= 0 && c.y >= 0).map(|c| ((c.x as u16, c.y as u16), c.z)).collect();

    let art = source.read(art_ini).ok().and_then(|b| IniDocument::parse(&b).ok());
    let Some(rules) = source.read(rules_ini).ok().and_then(|b| IniDocument::parse(&b).ok())
    else {
        return TerrainAnimBank {
            lighting: map.lighting.clone(),
            point_lights: map.point_lights.clone(),
            layers: Vec::new(),
        };
    };
    let obj_pal = source
        .read(theater_palette(map.theater))
        .ok()
        .and_then(|b| Palette::parse(&b).ok())
        .or_else(|| source.read("unittem.pal").ok().and_then(|b| Palette::parse(&b).ok()));
    let Some(obj_pal) = obj_pal
    else {
        return TerrainAnimBank {
            lighting: map.lighting.clone(),
            point_lights: map.point_lights.clone(),
            layers: Vec::new(),
        };
    };

    let ext = theater_tmp_extension(map.theater);
    let mut shp_cache: HashMap<String, ShpFile> = HashMap::new();
    let mut layers = Vec::new();

    for obj in &map.terrain_objects {
        if !is_yes(rules.get(&obj.name, "IsAnimated")) {
            continue;
        }
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
                frames.push(TileBlit {
                    width: 0,
                    height: 0,
                    offset_x: 0,
                    offset_y: 0,
                    rgba: Vec::new(),
                    shadow: None,
                });
                continue;
            }
            frames.push(frame_to_blit(frame, shp.width, shp.height, &obj_pal));
        }
        if frames.iter().all(|f| f.width == 0) {
            continue;
        }
        let cell_z = z_lookup.get(&(obj.x, obj.y)).copied().unwrap_or(0);
        layers.push(TerrainAnimLayer {
            x: obj.x,
            y: obj.y,
            cell_z,
            rate_ms: terrain_animation_rate_ms(anim_rate),
            frames,
            file,
            canvas_width: shp.width,
            canvas_height: shp.height,
        });
    }

    TerrainAnimBank {
        lighting: map.lighting.clone(),
        point_lights: map.point_lights.clone(),
        layers,
    }
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
        apply_rgba_tint(
            &mut painted.rgba,
            cell_tint_with_lights(&bank.lighting, layer.cell_z, layer.x, layer.y, &bank.point_lights),
        );
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

fn frame_to_blit(frame: &ra_assets::ShpFrame, shp_w: u16, shp_h: u16, pal: &Palette) -> TileBlit {
    TileBlit {
        width: u32::from(frame.frame_width),
        height: u32::from(frame.frame_height),
        offset_x: i32::from(frame.frame_x as i16) - i32::from(shp_w) / 2 + TILE_WIDTH / 2,
        offset_y: i32::from(frame.frame_y as i16) - i32::from(shp_h) / 2 + TILE_HEIGHT / 2 + TERRAIN_OBJECT_Y_FUDGE,
        rgba: frame.to_rgba(pal),
        shadow: None,
    }
}

fn is_yes(raw: Option<&str>) -> bool {
    raw.is_some_and(|v| v.eq_ignore_ascii_case("yes"))
}

fn parse_u32(raw: &str) -> Option<u32> {
    raw.trim().parse().ok()
}
