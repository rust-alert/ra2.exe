//! 地图建筑放置段 SHP 叠画（含 ActiveAnim 时钟）。

use std::collections::HashMap;

use ra_assets::{IniDocument, Palette, ShpFile};
use ra_types::AssetSource;

use crate::{
    MapEntityKind, MapInfo,
    compose::{TerrainImage, TileBlit, paint_cell_sprites},
    iso_math::TILE_WIDTH,
    theater::{new_theater_shp_name, theater_palette},
};

/// 建筑活动层绘制模式。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StructureAnimMode {
    /// 只画主体（不含 ActiveAnim）。
    BodyOnly,
    /// 主体 + 活动层；`clock_ms` 驱动 Rate/Loop 选帧。
    BodyAndAnims {
        /// 呈现时钟（毫秒）。
        clock_ms: u64,
    },
}

/// 预烘焙的单条建筑活动层（泵机 / 旗帜等）。
#[derive(Debug, Clone)]
pub struct StructureAnimLayer {
    /// 格子 X。
    pub x: u16,
    /// 格子 Y。
    pub y: u16,
    /// 格子高度（叠画用）。
    pub cell_z: u8,
    /// art `Rate`（毫秒/帧）。
    pub rate_ms: u32,
    /// `LoopStart`。
    pub loop_start: u16,
    /// `LoopEnd`（半开区间上界）。
    pub loop_end: u16,
    /// 已解码帧；下标 0 对应 `loop_start`。
    pub frames: Vec<TileBlit>,
}

/// 地图上全部建筑活动层（装载时烘焙，对局按时钟选帧）。
#[derive(Debug, Clone, Default)]
pub struct StructureAnimBank {
    /// 活动层列表。
    pub layers: Vec<StructureAnimLayer>,
}

impl StructureAnimBank {
    /// 是否有可播活动层。
    pub fn is_empty(&self) -> bool {
        self.layers.is_empty()
    }

    /// 当前时钟下各层帧号签名（用于跳过无变化的预览刷新）。
    pub fn frame_signature(&self, clock_ms: u64) -> u64 {
        let mut h: u64 = 0xcbf29ce484222325;
        for layer in &self.layers {
            let frame = structure_anim_frame(clock_ms, layer.rate_ms, layer.loop_start, layer.loop_end);
            h ^= u64::from(frame);
            h = h.wrapping_mul(0x100000001b3);
            h ^= u64::from(layer.x) << 16 | u64::from(layer.y);
            h = h.wrapping_mul(0x100000001b3);
        }
        h
    }
}

/// 按 art `Rate`（毫秒/帧）与 Loop 区间选取当前帧号。
///
/// `loop_end` 为半开上界；若 `loop_end <= loop_start` 则固定 `loop_start`。
pub fn structure_anim_frame(clock_ms: u64, rate_ms: u32, loop_start: u16, loop_end: u16) -> u16 {
    if loop_end <= loop_start {
        return loop_start;
    }
    let rate = u64::from(rate_ms.max(1));
    let len = u64::from(loop_end - loop_start);
    let idx = (clock_ms / rate) % len;
    loop_start + idx as u16
}

/// 叠画 `[Structures]`。`remap_owner(base, owner)` 返回房屋色调色板。
///
/// `BodyAndAnims` 时叠 `ActiveAnim` / `ActiveAnimTwo`（如油田旗帜 `CAOILD_F`）。
pub fn paint_map_structures(
    source: &dyn AssetSource,
    map: &MapInfo,
    image: &mut TerrainImage,
    art_ini: &str,
    remap_owner: &dyn Fn(&Palette, &str) -> Palette,
    mode: StructureAnimMode,
) -> usize {
    let (paint_body, clock_ms) = match mode {
        StructureAnimMode::BodyOnly => (true, None),
        StructureAnimMode::BodyAndAnims { clock_ms } => (true, Some(clock_ms)),
    };
    paint_map_structures_inner(source, map, image, art_ini, remap_owner, paint_body, clock_ms)
}

/// 收集建筑活动层并预解码全部循环帧（不含主体）。
pub fn collect_structure_anim_bank(
    source: &dyn AssetSource,
    map: &MapInfo,
    art_ini: &str,
    remap_owner: &dyn Fn(&Palette, &str) -> Palette,
) -> StructureAnimBank {
    let structures: Vec<_> = map.entities.iter().filter(|e| e.kind == MapEntityKind::Structure).collect();
    if structures.is_empty() {
        return StructureAnimBank::default();
    }

    let z_lookup: HashMap<(u16, u16), u8> =
        map.cells.iter().filter(|c| c.x >= 0 && c.y >= 0).map(|c| ((c.x as u16, c.y as u16), c.z)).collect();

    let art = source.read(art_ini).ok().and_then(|b| IniDocument::parse(&b).ok());
    let Some(obj_pal) = load_object_palette(source, map)
    else {
        return StructureAnimBank::default();
    };

    let mut shp_cache: HashMap<String, ShpFile> = HashMap::new();
    let mut layers = Vec::new();

    for ent in structures {
        let art_section = resolve_art_section(art.as_ref(), &ent.type_id);
        let remapable = is_remapable(art.as_ref(), &art_section, true);
        let cell_z = z_lookup.get(&(ent.x, ent.y)).copied().unwrap_or(0);

        for (anim_key, z_key) in [("ActiveAnim", "ActiveAnimZAdjust"), ("ActiveAnimTwo", "ActiveAnimTwoZAdjust")] {
            let Some(anim_name) = art.as_ref().and_then(|a| a.get(&art_section, anim_key)).map(str::to_ascii_uppercase)
            else {
                continue;
            };
            let z_adjust = art.as_ref().and_then(|a| a.get(&art_section, z_key)).and_then(parse_i32).unwrap_or(0);
            let anim_image = art.as_ref().and_then(|a| a.get(&anim_name, "Image")).unwrap_or(anim_name.as_str()).to_ascii_uppercase();
            let anim_new_theater =
                art.as_ref().and_then(|a| a.get(&anim_name, "NewTheater")).is_some_and(|v| v.eq_ignore_ascii_case("yes"));
            let loop_start = art
                .as_ref()
                .and_then(|a| a.get(&anim_name, "LoopStart").or_else(|| a.get(&anim_name, "Start")))
                .and_then(parse_u16)
                .unwrap_or(0);
            let loop_end = art.as_ref().and_then(|a| a.get(&anim_name, "LoopEnd")).and_then(parse_u16).unwrap_or(loop_start + 1);
            let rate_ms = art.as_ref().and_then(|a| a.get(&anim_name, "Rate")).and_then(parse_u32).unwrap_or(300);
            let anim_remapable = art
                .as_ref()
                .and_then(|a| a.get(&anim_name, "Remapable"))
                .map(|v| !v.eq_ignore_ascii_case("no"))
                .unwrap_or(remapable);
            let anim_pal = if anim_remapable { remap_owner(&obj_pal, &ent.owner) } else { obj_pal.clone() };

            let Some(shp) = load_shp(source, map, &anim_image, anim_new_theater, &mut shp_cache)
            else {
                continue;
            };
            let end = if loop_end > loop_start { loop_end } else { loop_start.saturating_add(1) };
            let mut frames = Vec::with_capacity(usize::from(end.saturating_sub(loop_start)));
            for frame_idx in loop_start..end {
                let Some(blit) = frame_to_blit(shp, frame_idx, z_adjust, &anim_pal)
                else {
                    // 空帧占位，保持下标对齐。
                    frames.push(TileBlit { width: 0, height: 0, offset_x: 0, offset_y: z_adjust, rgba: Vec::new() });
                    continue;
                };
                frames.push(blit);
            }
            if frames.iter().all(|f| f.width == 0) {
                continue;
            }
            layers.push(StructureAnimLayer {
                x: ent.x,
                y: ent.y,
                cell_z,
                rate_ms,
                loop_start,
                loop_end: end,
                frames,
            });
        }
    }

    StructureAnimBank { layers }
}

/// 按时钟把活动层叠到地形图上。
pub fn paint_structure_anim_bank(image: &mut TerrainImage, bank: &StructureAnimBank, clock_ms: u64) -> usize {
    if bank.layers.is_empty() {
        return 0;
    }
    let mut items: Vec<(u16, u16, TileBlit)> = Vec::with_capacity(bank.layers.len());
    for layer in &bank.layers {
        let frame_no = structure_anim_frame(clock_ms, layer.rate_ms, layer.loop_start, layer.loop_end);
        let local = usize::from(frame_no.saturating_sub(layer.loop_start));
        let Some(blit) = layer.frames.get(local)
        else {
            continue;
        };
        if blit.width == 0 || blit.height == 0 {
            continue;
        }
        items.push((layer.x, layer.y, blit.clone()));
    }
    let z_at = |x: u16, y: u16| {
        bank.layers.iter().find(|l| l.x == x && l.y == y).map(|l| l.cell_z).unwrap_or(0)
    };
    paint_cell_sprites(image, &items, z_at)
}

/// 在已有 RGBA 预览上叠活动层（保留原点）。
pub fn paint_structure_anims_onto_rgba(
    image: &mut image::RgbaImage,
    origin_x: i32,
    origin_y: i32,
    bank: &StructureAnimBank,
    clock_ms: u64,
) -> usize {
    let mut terrain = TerrainImage { image: std::mem::take(image), drawn: 0, origin_x, origin_y };
    let n = paint_structure_anim_bank(&mut terrain, bank, clock_ms);
    *image = terrain.image;
    n
}

/// 建筑一次性 Buildup 序列（MCV 展开 / 放置建造等）。
#[derive(Debug, Clone)]
pub struct StructureBuildupClip {
    /// 格子 X。
    pub x: u16,
    /// 格子 Y。
    pub y: u16,
    /// 格子高度。
    pub cell_z: u8,
    /// 毫秒/帧。
    pub rate_ms: u32,
    /// 按播放顺序的已解码帧。
    pub frames: Vec<TileBlit>,
}

impl StructureBuildupClip {
    /// 按已过毫秒取当前帧下标；播完返回 `None`。
    pub fn frame_at(&self, elapsed_ms: u64) -> Option<usize> {
        buildup_frame_index(elapsed_ms, self.rate_ms, self.frames.len())
    }
}

/// Buildup 一次性选帧：`elapsed / rate`；越界表示播完。
pub fn buildup_frame_index(elapsed_ms: u64, rate_ms: u32, frame_count: usize) -> Option<usize> {
    if frame_count == 0 {
        return None;
    }
    let rate = u64::from(rate_ms.max(1));
    let idx = (elapsed_ms / rate) as usize;
    if idx >= frame_count {
        None
    } else {
        Some(idx)
    }
}

/// 从 art `Buildup=` 装入一次性展开序列。无 `Buildup` 或资源缺失时返回 `None`。
pub fn load_structure_buildup_clip(
    source: &dyn AssetSource,
    map: &MapInfo,
    art_ini: &str,
    type_id: &str,
    owner: &str,
    x: u16,
    y: u16,
    remap_owner: &dyn Fn(&Palette, &str) -> Palette,
) -> Option<StructureBuildupClip> {
    let art = source.read(art_ini).ok().and_then(|b| IniDocument::parse(&b).ok())?;
    let art_section = resolve_art_section(Some(&art), type_id);
    let buildup_key = art.get(&art_section, "Buildup")?.to_ascii_uppercase();
    let parent_new_theater = art.get(&art_section, "NewTheater").is_some_and(|v| v.eq_ignore_ascii_case("yes"));
    // 无独立 `[GACNSTMK]` 段时沿用建筑段的 `NewTheater`，文件名即 `Buildup` 键。
    let image_key = art.get(&buildup_key, "Image").unwrap_or(buildup_key.as_str()).to_ascii_uppercase();
    let new_theater = art
        .get(&buildup_key, "NewTheater")
        .map(|v| v.eq_ignore_ascii_case("yes"))
        .unwrap_or(parent_new_theater);
    let rate_ms = art.get(&buildup_key, "Rate").and_then(parse_u32).unwrap_or(100);
    let remapable = is_remapable(Some(&art), &art_section, true);
    let obj_pal = load_object_palette(source, map)?;
    let pal = if remapable { remap_owner(&obj_pal, owner) } else { obj_pal };
    let mut shp_cache: HashMap<String, ShpFile> = HashMap::new();
    let shp = load_shp(source, map, &image_key, new_theater, &mut shp_cache)?;
    let mut frames = Vec::with_capacity(shp.frames.len());
    for i in 0..shp.frames.len() {
        let Some(blit) = frame_to_blit(shp, i as u16, 0, &pal)
        else {
            continue;
        };
        if blit.width == 0 || blit.height == 0 {
            continue;
        }
        frames.push(blit);
    }
    if frames.is_empty() {
        return None;
    }
    let cell_z = map
        .cells
        .iter()
        .find(|c| c.x == x as i16 && c.y == y as i16)
        .map(|c| c.z)
        .unwrap_or(0);
    Some(StructureBuildupClip { x, y, cell_z, rate_ms, frames })
}

/// 把 Buildup 某一帧叠到 RGBA 预览。
pub fn paint_structure_buildup_onto_rgba(
    image: &mut image::RgbaImage,
    origin_x: i32,
    origin_y: i32,
    clip: &StructureBuildupClip,
    frame_idx: usize,
) -> bool {
    let Some(blit) = clip.frames.get(frame_idx)
    else {
        return false;
    };
    let mut terrain = TerrainImage { image: std::mem::take(image), drawn: 0, origin_x, origin_y };
    let items = vec![(clip.x, clip.y, blit.clone())];
    let z = clip.cell_z;
    paint_cell_sprites(&mut terrain, &items, |_, _| z);
    *image = terrain.image;
    true
}

/// 在已有 RGBA 上叠建筑主体（`BodyOnly`）。
pub fn paint_structures_onto_rgba(
    source: &dyn AssetSource,
    map: &MapInfo,
    image: &mut image::RgbaImage,
    origin_x: i32,
    origin_y: i32,
    art_ini: &str,
    remap_owner: &dyn Fn(&Palette, &str) -> Palette,
) -> usize {
    let mut terrain = TerrainImage { image: std::mem::take(image), drawn: 0, origin_x, origin_y };
    let n = paint_map_structures(source, map, &mut terrain, art_ini, remap_owner, StructureAnimMode::BodyOnly);
    *image = terrain.image;
    n
}

fn paint_map_structures_inner(
    source: &dyn AssetSource,
    map: &MapInfo,
    image: &mut TerrainImage,
    art_ini: &str,
    remap_owner: &dyn Fn(&Palette, &str) -> Palette,
    paint_body: bool,
    anim_clock_ms: Option<u64>,
) -> usize {
    let structures: Vec<_> = map.entities.iter().filter(|e| e.kind == MapEntityKind::Structure).collect();
    if structures.is_empty() {
        return 0;
    }

    let z_lookup: HashMap<(u16, u16), u8> =
        map.cells.iter().filter(|c| c.x >= 0 && c.y >= 0).map(|c| ((c.x as u16, c.y as u16), c.z)).collect();
    let z_at = |x: u16, y: u16| z_lookup.get(&(x, y)).copied().unwrap_or(0);

    let art = source.read(art_ini).ok().and_then(|b| IniDocument::parse(&b).ok());
    let Some(obj_pal) = load_object_palette(source, map)
    else {
        return 0;
    };

    let mut shp_cache: HashMap<String, ShpFile> = HashMap::new();
    let mut blit_cache: HashMap<(String, String, u16, i32), TileBlit> = HashMap::new();
    let mut items: Vec<(u16, u16, TileBlit)> = Vec::new();

    for ent in structures {
        let art_section = resolve_art_section(art.as_ref(), &ent.type_id);
        let remapable = is_remapable(art.as_ref(), &art_section, true);
        let pal = if remapable { remap_owner(&obj_pal, &ent.owner) } else { obj_pal.clone() };

        if paint_body {
            let body_key = art.as_ref().and_then(|a| a.get(&art_section, "Image")).unwrap_or(art_section.as_str()).to_ascii_uppercase();
            let body_new_theater =
                art.as_ref().and_then(|a| a.get(&art_section, "NewTheater")).is_some_and(|v| v.eq_ignore_ascii_case("yes"));
            if let Some(blit) =
                load_structure_blit(source, map, &body_key, body_new_theater, 0, 0, &pal, &mut shp_cache, &mut blit_cache, &ent.owner)
            {
                items.push((ent.x, ent.y, blit));
            }
        }

        let Some(clock_ms) = anim_clock_ms
        else {
            continue;
        };
        for (anim_key, z_key) in [("ActiveAnim", "ActiveAnimZAdjust"), ("ActiveAnimTwo", "ActiveAnimTwoZAdjust")] {
            let Some(anim_name) = art.as_ref().and_then(|a| a.get(&art_section, anim_key)).map(str::to_ascii_uppercase)
            else {
                continue;
            };
            let z_adjust = art.as_ref().and_then(|a| a.get(&art_section, z_key)).and_then(parse_i32).unwrap_or(0);
            let anim_image = art.as_ref().and_then(|a| a.get(&anim_name, "Image")).unwrap_or(anim_name.as_str()).to_ascii_uppercase();
            let anim_new_theater =
                art.as_ref().and_then(|a| a.get(&anim_name, "NewTheater")).is_some_and(|v| v.eq_ignore_ascii_case("yes"));
            let loop_start = art
                .as_ref()
                .and_then(|a| a.get(&anim_name, "LoopStart").or_else(|| a.get(&anim_name, "Start")))
                .and_then(parse_u16)
                .unwrap_or(0);
            let loop_end = art.as_ref().and_then(|a| a.get(&anim_name, "LoopEnd")).and_then(parse_u16).unwrap_or(loop_start + 1);
            let rate_ms = art.as_ref().and_then(|a| a.get(&anim_name, "Rate")).and_then(parse_u32).unwrap_or(300);
            let frame_idx = structure_anim_frame(clock_ms, rate_ms, loop_start, loop_end);
            let anim_remapable = art
                .as_ref()
                .and_then(|a| a.get(&anim_name, "Remapable"))
                .map(|v| !v.eq_ignore_ascii_case("no"))
                .unwrap_or(remapable);
            let anim_pal = if anim_remapable { remap_owner(&obj_pal, &ent.owner) } else { obj_pal.clone() };
            if let Some(blit) = load_structure_blit(
                source,
                map,
                &anim_image,
                anim_new_theater,
                frame_idx,
                z_adjust,
                &anim_pal,
                &mut shp_cache,
                &mut blit_cache,
                &ent.owner,
            ) {
                items.push((ent.x, ent.y, blit));
            }
        }
    }

    paint_cell_sprites(image, &items, z_at)
}

fn load_object_palette(source: &dyn AssetSource, map: &MapInfo) -> Option<Palette> {
    source
        .read("unittem.pal")
        .ok()
        .and_then(|b| Palette::parse(&b).ok())
        .or_else(|| source.read(theater_palette(map.theater)).ok().and_then(|b| Palette::parse(&b).ok()))
}

fn resolve_art_section(art: Option<&IniDocument>, type_id: &str) -> String {
    art.and_then(|a| {
        let image_key = a.get(type_id, "Image").unwrap_or(type_id);
        if a.section(image_key).is_some() {
            Some(image_key.to_ascii_uppercase())
        } else if a.section(type_id).is_some() {
            Some(type_id.to_ascii_uppercase())
        } else {
            None
        }
    })
    .unwrap_or_else(|| type_id.to_ascii_uppercase())
}

fn is_remapable(art: Option<&IniDocument>, section: &str, default_yes: bool) -> bool {
    match art.and_then(|a| a.get(section, "Remapable")) {
        Some(v) if v.eq_ignore_ascii_case("no") => false,
        Some(_) => true,
        None => default_yes,
    }
}

fn load_shp<'a>(
    source: &dyn AssetSource,
    map: &MapInfo,
    image_key: &str,
    new_theater: bool,
    shp_cache: &'a mut HashMap<String, ShpFile>,
) -> Option<&'a ShpFile> {
    let candidates = if new_theater {
        vec![new_theater_shp_name(image_key, map.theater), format!("{}.shp", image_key.to_ascii_lowercase())]
    } else {
        vec![format!("{}.shp", image_key.to_ascii_lowercase()), new_theater_shp_name(image_key, map.theater)]
    };
    let mut loaded: Option<String> = None;
    for file in &candidates {
        if shp_cache.contains_key(file) {
            loaded = Some(file.clone());
            break;
        }
        let Ok(bytes) = source.read(file)
        else {
            continue;
        };
        let Ok(shp) = ShpFile::parse(&bytes)
        else {
            continue;
        };
        shp_cache.insert(file.clone(), shp);
        loaded = Some(file.clone());
        break;
    }
    loaded.and_then(move |file| shp_cache.get(&file))
}

fn frame_to_blit(shp: &ShpFile, frame_idx: u16, z_adjust: i32, pal: &Palette) -> Option<TileBlit> {
    let frame = shp.frames.get(usize::from(frame_idx))?;
    if frame.frame_width == 0 || frame.frame_height == 0 {
        return None;
    }
    // TS/RA2 建筑 SHP：`frame_x/y` 是相对整幅画布的裁切原点。
    // 叠画相对 `iso_to_screen`（钻石包围盒左上）时，画布中心落在「箱顶边中点」
    // `(+TILE_WIDTH/2, 0)`，再加裁切偏移。单位/选中环在钻石中心 `(+30,+15)`，
    // 建筑艺术锚点比其高半格（Y 减 `TILE_HEIGHT/2`）。
    Some(TileBlit {
        width: u32::from(frame.frame_width),
        height: u32::from(frame.frame_height),
        offset_x: i32::from(frame.frame_x as i16) - i32::from(shp.width) / 2 + TILE_WIDTH / 2,
        offset_y: i32::from(frame.frame_y as i16) - i32::from(shp.height) / 2 + z_adjust,
        rgba: frame.to_rgba(pal),
    })
}

fn load_structure_blit(
    source: &dyn AssetSource,
    map: &MapInfo,
    image_key: &str,
    new_theater: bool,
    frame_idx: u16,
    z_adjust: i32,
    pal: &Palette,
    shp_cache: &mut HashMap<String, ShpFile>,
    blit_cache: &mut HashMap<(String, String, u16, i32), TileBlit>,
    owner: &str,
) -> Option<TileBlit> {
    let cache_key = (image_key.to_owned(), owner.to_owned(), frame_idx, z_adjust);
    if let Some(blit) = blit_cache.get(&cache_key) {
        return Some(blit.clone());
    }
    let shp = load_shp(source, map, image_key, new_theater, shp_cache)?;
    let blit = frame_to_blit(shp, frame_idx, z_adjust, pal)?;
    blit_cache.insert(cache_key, blit.clone());
    Some(blit)
}

fn parse_i32(raw: &str) -> Option<i32> {
    raw.trim().parse().ok()
}

fn parse_u16(raw: &str) -> Option<u16> {
    raw.trim().parse().ok()
}

fn parse_u32(raw: &str) -> Option<u32> {
    raw.trim().parse().ok()
}
