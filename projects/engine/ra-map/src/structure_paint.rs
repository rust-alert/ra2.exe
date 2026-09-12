//! 地图建筑放置段 SHP 叠画（含 ActiveAnim 时钟与受损燃烧）。

use std::collections::HashMap;

use ra_assets::{HvaFile, IniDocument, Palette, ShpFile, VplFile, VxlFile, VxlLayerPose, rasterize_vxl_layer_poses, shp_body_frame_count};
use ra_types::{AssetSource, ImageName, TechnoName};
use serde::Deserialize;
use serde::de::Deserializer;

use crate::{
    MapEntity, MapEntityKind, MapInfo,
    compose::{TerrainImage, TileBlit, paint_cell_sprites},
    iso_math::TILE_WIDTH,
    lighting::{PointLight, apply_rgba_tint, cell_tint_with_lights},
    structure_damage::{damaged_body_frame, structure_tech_level},
    theater::{new_theater_shp_name, theater_palette},
};

/// 建筑循环活动层键：常态 / 受损 / ZAdjust。含 `IdleAnim`（科技前哨收回臂等）。
const STRUCTURE_LOOP_ANIM_KEYS: &[(&str, &str, &str)] = &[
    ("ActiveAnim", "ActiveAnimDamaged", "ActiveAnimZAdjust"),
    ("ActiveAnimTwo", "ActiveAnimTwoDamaged", "ActiveAnimTwoZAdjust"),
    ("IdleAnim", "IdleAnimDamaged", "IdleAnimZAdjust"),
    ("IdleAnimTwo", "IdleAnimTwoDamaged", "IdleAnimTwoZAdjust"),
];

/// 建筑类型叠画主体提示（按 `type_id` 去重一次）。
#[derive(Debug, Clone)]
struct StructureTypePaintHints {
    remapable: bool,
    body_key: String,
    body_new_theater: bool,
    bib_key: Option<String>,
    bib_new_theater: bool,
    tech_level: i32,
    /// rules `TurretAnimIsVoxel` 炮塔体素（缺则跳过）。
    turret_voxel: Option<StructureTurretVoxelHints>,
    /// art `DamageFireOffset0..7`（槽位, x, y）。
    fire_offsets: Vec<(u8, i32, i32)>,
    /// art `Buildup=` 一次性展开序列（缺则 `None`）。
    buildup: Option<StructureBuildupHints>,
    /// 与 [`STRUCTURE_LOOP_ANIM_KEYS`] 对齐：`(normal, damaged)` 活动层节名。
    loop_anims: Vec<(Option<String>, Option<String>)>,
}

/// 建筑类型叠画提示表（跨多次 paint / anim-bank 调用复用，避免重复扫 art/rules）。
#[derive(Debug, Clone, Default)]
pub struct StructurePaintHintTable {
    by_type: HashMap<TechnoName, StructureTypePaintHints>,
}

impl StructurePaintHintTable {
    /// 确保表中含该类型提示（已有则跳过 INI 扫描）。
    pub fn ensure(&mut self, art_rules: &crate::ArtRules, type_id: &TechnoName) {
        if self.by_type.contains_key(type_id) {
            return;
        }
        let hint = structure_type_paint_hints(art_rules.art.as_ref(), art_rules.rules.as_ref(), type_id.as_str());
        self.by_type.insert(type_id.clone(), hint);
    }

    /// 为实体列表补齐提示。
    pub fn ensure_entities(&mut self, art_rules: &crate::ArtRules, structures: &[&MapEntity]) {
        for ent in structures {
            self.ensure(art_rules, &ent.type_id);
        }
    }

    fn get(&self, type_id: &TechnoName) -> Option<&StructureTypePaintHints> {
        self.by_type.get(type_id)
    }
}

/// 建筑炮塔体素叠画提示。
#[derive(Debug, Clone)]
struct StructureTurretVoxelHints {
    stem: String,
    anim_x: i32,
    anim_y: i32,
}

/// 建筑 Buildup 叠画提示。
#[derive(Debug, Clone)]
struct StructureBuildupHints {
    image_key: String,
    new_theater: bool,
    rate_ms: u32,
}

fn structure_type_paint_hints(art: Option<&IniDocument>, rules: Option<&IniDocument>, type_id: &str) -> StructureTypePaintHints {
    let art_section = resolve_art_section(art, type_id);
    let body = structure_body_art_fields(art, type_id, &art_section);
    let remapable = body.remapable.unwrap_or(true);
    let body_new_theater = body.new_theater.unwrap_or(false);
    let bib_key = body
        .bib_shape
        .as_ref()
        .filter(|n| !n.is_empty())
        .map(|n| n.as_str().to_string());
    let bib_new_theater = match bib_key.as_ref() {
        Some(bib) => art
            .and_then(|a| a.section(bib))
            .and_then(|s| s.deserialize::<BibShapeSectionFields>().ok())
            .and_then(|f| f.new_theater)
            .unwrap_or(body_new_theater),
        None => body_new_theater,
    };
    let body_key = body
        .image
        .as_ref()
        .filter(|n| !n.is_empty())
        .map(|n| n.as_str().to_string())
        .unwrap_or_else(|| art_section.trim().to_ascii_uppercase());
    StructureTypePaintHints {
        remapable,
        body_key,
        body_new_theater,
        bib_key,
        bib_new_theater,
        tech_level: structure_tech_level(rules, type_id),
        turret_voxel: structure_turret_voxel_hints(rules, type_id),
        fire_offsets: structure_damage_fire_offsets(&body),
        buildup: structure_buildup_hints(art, body.buildup.as_ref().map(|n| n.as_str()), body_new_theater),
        loop_anims: structure_loop_anim_names(&body),
    }
}

fn structure_body_art_fields(art: Option<&IniDocument>, type_id: &str, art_section: &str) -> StructureBodyArtFields {
    let art = match art {
        Some(a) => a,
        None => return StructureBodyArtFields::default(),
    };
    let type_fields = art
        .section(type_id)
        .and_then(|s| s.deserialize::<StructureBodyArtFields>().ok())
        .unwrap_or_default();
    if art_section.eq_ignore_ascii_case(type_id) {
        return type_fields;
    }
    let section_fields = art
        .section(art_section)
        .and_then(|s| s.deserialize::<StructureBodyArtFields>().ok())
        .unwrap_or_default();
    // 类型节优先（`DamageFireOffset*` / 活动层名常挂在类型节），缺键再回退 `Image=` 目标节。
    StructureBodyArtFields {
        image: type_fields.image.or(section_fields.image),
        new_theater: type_fields.new_theater.or(section_fields.new_theater),
        remapable: type_fields.remapable.or(section_fields.remapable),
        bib_shape: type_fields.bib_shape.or(section_fields.bib_shape),
        buildup: type_fields.buildup.or(section_fields.buildup),
        active_anim: type_fields.active_anim.or(section_fields.active_anim),
        active_anim_damaged: type_fields.active_anim_damaged.or(section_fields.active_anim_damaged),
        active_anim_two: type_fields.active_anim_two.or(section_fields.active_anim_two),
        active_anim_two_damaged: type_fields.active_anim_two_damaged.or(section_fields.active_anim_two_damaged),
        idle_anim: type_fields.idle_anim.or(section_fields.idle_anim),
        idle_anim_damaged: type_fields.idle_anim_damaged.or(section_fields.idle_anim_damaged),
        idle_anim_two: type_fields.idle_anim_two.or(section_fields.idle_anim_two),
        idle_anim_two_damaged: type_fields.idle_anim_two_damaged.or(section_fields.idle_anim_two_damaged),
        damage_fire_offset0: type_fields.damage_fire_offset0.or(section_fields.damage_fire_offset0),
        damage_fire_offset1: type_fields.damage_fire_offset1.or(section_fields.damage_fire_offset1),
        damage_fire_offset2: type_fields.damage_fire_offset2.or(section_fields.damage_fire_offset2),
        damage_fire_offset3: type_fields.damage_fire_offset3.or(section_fields.damage_fire_offset3),
        damage_fire_offset4: type_fields.damage_fire_offset4.or(section_fields.damage_fire_offset4),
        damage_fire_offset5: type_fields.damage_fire_offset5.or(section_fields.damage_fire_offset5),
        damage_fire_offset6: type_fields.damage_fire_offset6.or(section_fields.damage_fire_offset6),
        damage_fire_offset7: type_fields.damage_fire_offset7.or(section_fields.damage_fire_offset7),
    }
}

#[derive(Debug, Default, Deserialize)]
struct StructureBodyArtFields {
    #[serde(rename = "Image")]
    image: Option<ImageName>,
    #[serde(rename = "NewTheater")]
    new_theater: Option<bool>,
    #[serde(rename = "Remapable")]
    remapable: Option<bool>,
    #[serde(rename = "BibShape")]
    bib_shape: Option<ImageName>,
    #[serde(rename = "Buildup")]
    buildup: Option<ImageName>,
    #[serde(rename = "ActiveAnim")]
    active_anim: Option<ImageName>,
    #[serde(rename = "ActiveAnimDamaged")]
    active_anim_damaged: Option<ImageName>,
    #[serde(rename = "ActiveAnimTwo")]
    active_anim_two: Option<ImageName>,
    #[serde(rename = "ActiveAnimTwoDamaged")]
    active_anim_two_damaged: Option<ImageName>,
    #[serde(rename = "IdleAnim")]
    idle_anim: Option<ImageName>,
    #[serde(rename = "IdleAnimDamaged")]
    idle_anim_damaged: Option<ImageName>,
    #[serde(rename = "IdleAnimTwo")]
    idle_anim_two: Option<ImageName>,
    #[serde(rename = "IdleAnimTwoDamaged")]
    idle_anim_two_damaged: Option<ImageName>,
    #[serde(rename = "DamageFireOffset0", default, deserialize_with = "de_opt_damage_fire_offset")]
    damage_fire_offset0: Option<(i32, i32)>,
    #[serde(rename = "DamageFireOffset1", default, deserialize_with = "de_opt_damage_fire_offset")]
    damage_fire_offset1: Option<(i32, i32)>,
    #[serde(rename = "DamageFireOffset2", default, deserialize_with = "de_opt_damage_fire_offset")]
    damage_fire_offset2: Option<(i32, i32)>,
    #[serde(rename = "DamageFireOffset3", default, deserialize_with = "de_opt_damage_fire_offset")]
    damage_fire_offset3: Option<(i32, i32)>,
    #[serde(rename = "DamageFireOffset4", default, deserialize_with = "de_opt_damage_fire_offset")]
    damage_fire_offset4: Option<(i32, i32)>,
    #[serde(rename = "DamageFireOffset5", default, deserialize_with = "de_opt_damage_fire_offset")]
    damage_fire_offset5: Option<(i32, i32)>,
    #[serde(rename = "DamageFireOffset6", default, deserialize_with = "de_opt_damage_fire_offset")]
    damage_fire_offset6: Option<(i32, i32)>,
    #[serde(rename = "DamageFireOffset7", default, deserialize_with = "de_opt_damage_fire_offset")]
    damage_fire_offset7: Option<(i32, i32)>,
}

#[derive(Debug, Default, Deserialize)]
struct BibShapeSectionFields {
    #[serde(rename = "NewTheater")]
    new_theater: Option<bool>,
}

fn structure_loop_anim_names(body: &StructureBodyArtFields) -> Vec<(Option<String>, Option<String>)> {
    let name = |s: &Option<ImageName>| s.as_ref().filter(|n| !n.is_empty()).map(|n| n.as_str().to_string());
    vec![
        (name(&body.active_anim), name(&body.active_anim_damaged)),
        (name(&body.active_anim_two), name(&body.active_anim_two_damaged)),
        (name(&body.idle_anim), name(&body.idle_anim_damaged)),
        (name(&body.idle_anim_two), name(&body.idle_anim_two_damaged)),
    ]
}

fn pick_structure_loop_anim_name(slot: &(Option<String>, Option<String>), yellow: bool) -> Option<&str> {
    if yellow {
        slot.1.as_deref().or(slot.0.as_deref())
    } else {
        slot.0.as_deref()
    }
}

fn structure_buildup_hints(art: Option<&IniDocument>, buildup: Option<&str>, parent_new_theater: bool) -> Option<StructureBuildupHints> {
    let art = art?;
    let buildup_key = buildup?.trim().to_ascii_uppercase();
    if buildup_key.is_empty() {
        return None;
    }
    let fields = art
        .section(&buildup_key)
        .and_then(|s| s.deserialize::<BuildupSectionFields>().ok())
        .unwrap_or_default();
    // 无独立 Buildup 段时沿用建筑段的 `NewTheater`，文件名即 `Buildup` 键。
    let image_key = fields
        .image
        .as_ref()
        .filter(|n| !n.is_empty())
        .map(|n| n.as_str().to_string())
        .unwrap_or(buildup_key);
    let new_theater = fields.new_theater.unwrap_or(parent_new_theater);
    let rate_ms = fields.rate_ms.unwrap_or(100);
    Some(StructureBuildupHints { image_key, new_theater, rate_ms })
}

#[derive(Debug, Default, Deserialize)]
struct BuildupSectionFields {
    #[serde(rename = "Image")]
    image: Option<ImageName>,
    #[serde(rename = "NewTheater")]
    new_theater: Option<bool>,
    #[serde(rename = "Rate")]
    rate_ms: Option<u32>,
}

fn structure_damage_fire_offsets(body: &StructureBodyArtFields) -> Vec<(u8, i32, i32)> {
    let offsets = [
        body.damage_fire_offset0,
        body.damage_fire_offset1,
        body.damage_fire_offset2,
        body.damage_fire_offset3,
        body.damage_fire_offset4,
        body.damage_fire_offset5,
        body.damage_fire_offset6,
        body.damage_fire_offset7,
    ];
    let mut out = Vec::new();
    for (i, offset) in offsets.into_iter().enumerate() {
        let Some((ox, oy)) = offset else {
            continue;
        };
        out.push((i as u8, ox, oy));
    }
    out
}

fn de_opt_damage_fire_offset<'de, D>(deserializer: D) -> Result<Option<(i32, i32)>, D::Error>
where
    D: Deserializer<'de>,
{
    // 非法偏移软跳过（与旧 `parse_damage_fire_offset` 失败行为一致）。
    match <(i32, i32)>::deserialize(deserializer) {
        Ok(xy) => Ok(Some(xy)),
        Err(_) => Ok(None),
    }
}

fn structure_turret_voxel_hints(rules: Option<&IniDocument>, type_id: &str) -> Option<StructureTurretVoxelHints> {
    let rules = rules?;
    let fields = rules
        .section(type_id)
        .and_then(|s| s.deserialize::<TurretVoxelSectionFields>().ok())
        .unwrap_or_default();
    if !fields.is_voxel.unwrap_or(false) {
        return None;
    }
    let stem = fields.anim.as_deref()?.trim().to_ascii_lowercase();
    if stem.is_empty() {
        return None;
    }
    Some(StructureTurretVoxelHints {
        stem,
        anim_x: fields.anim_x.unwrap_or(0),
        anim_y: fields.anim_y.unwrap_or(0),
    })
}

#[derive(Debug, Default, Deserialize)]
struct TurretVoxelSectionFields {
    #[serde(rename = "TurretAnimIsVoxel")]
    is_voxel: Option<bool>,
    #[serde(rename = "TurretAnim")]
    anim: Option<String>,
    #[serde(rename = "TurretAnimX")]
    anim_x: Option<i32>,
    #[serde(rename = "TurretAnimY")]
    anim_y: Option<i32>,
}

fn collect_structure_type_paint_hints(
    hints: &mut StructurePaintHintTable,
    art_rules: &crate::ArtRules,
    structures: &[&MapEntity],
) {
    hints.ensure_entities(art_rules, structures);
}

/// 活动层 / 火焰 anim 节提示（按 anim 节名去重一次）。
#[derive(Debug, Clone)]
struct StructureAnimSectionHints {
    image_key: String,
    new_theater: bool,
    loop_start: u16,
    loop_end: u16,
    rate_ms: u32,
    /// `None` 表示沿用建筑默认 `Remapable`。
    remapable_override: Option<bool>,
}

fn structure_anim_section_hints(art: Option<&IniDocument>, anim_name: &str, default_rate_ms: u32) -> StructureAnimSectionHints {
    let fields = art
        .and_then(|a| a.section(anim_name))
        .and_then(|s| s.deserialize::<AnimSectionFields>().ok())
        .unwrap_or_default();
    let image_key = fields
        .image
        .as_ref()
        .filter(|n| !n.is_empty())
        .map(|n| n.as_str().to_string())
        .unwrap_or_else(|| anim_name.trim().to_ascii_uppercase());
    let loop_start = fields.loop_start.or(fields.start).unwrap_or(0);
    let loop_end = fields.loop_end.unwrap_or(loop_start.saturating_add(1));
    StructureAnimSectionHints {
        image_key,
        new_theater: fields.new_theater.unwrap_or(false),
        loop_start,
        loop_end,
        rate_ms: fields.rate_ms.unwrap_or(default_rate_ms),
        remapable_override: fields.remapable,
    }
}

#[derive(Debug, Default, Deserialize)]
struct AnimSectionFields {
    #[serde(rename = "Image")]
    image: Option<ImageName>,
    #[serde(rename = "NewTheater")]
    new_theater: Option<bool>,
    #[serde(rename = "LoopStart")]
    loop_start: Option<u16>,
    #[serde(rename = "Start")]
    start: Option<u16>,
    #[serde(rename = "LoopEnd")]
    loop_end: Option<u16>,
    #[serde(rename = "Rate")]
    rate_ms: Option<u32>,
    #[serde(rename = "Remapable")]
    remapable: Option<bool>,
}

fn cached_anim_section_hint<'a>(
    cache: &'a mut HashMap<String, StructureAnimSectionHints>,
    art: Option<&IniDocument>,
    anim_name: &str,
    default_rate_ms: u32,
) -> &'a StructureAnimSectionHints {
    if !cache.contains_key(anim_name) {
        cache.insert(anim_name.to_string(), structure_anim_section_hints(art, anim_name, default_rate_ms));
    }
    cache.get(anim_name).expect("just inserted")
}

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
#[derive(Debug, Clone)]
pub struct StructureAnimBank {
    /// 收集时的地图 `[Lighting]`（播帧时按格 tint）。
    pub lighting: crate::LightingConfig,
    /// 收集时的点光源。
    pub point_lights: Vec<PointLight>,
    /// 活动层列表。
    pub layers: Vec<StructureAnimLayer>,
}

impl Default for StructureAnimBank {
    fn default() -> Self {
        Self { lighting: crate::LightingConfig::default(), point_lights: Vec::new(), layers: Vec::new() }
    }
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
/// `BodyAndAnims` 时叠 `ActiveAnim` / `IdleAnim` 等循环层，并画 `BibShape` 与体素 `TurretAnim`。
/// `rules_ini` 提供 `ConditionYellow` / `ConditionRed` / `DamageFireTypes` / 炮塔偏移。
/// 黄血起火并切 `*Damaged`；主体受损帧按 `TechLevel` 区分军建黄档与平民红档。
/// 不自动叠 `SpecialAnim*`（修理臂等状态机层，需仿真态才播）。
///
/// 返回 `(成功叠画件数含 bib/炮塔/活动层, 主体缺失占位色块数)`。
pub fn paint_map_structures(
    source: &dyn AssetSource,
    map: &MapInfo,
    image: &mut TerrainImage,
    art_rules: &crate::ArtRules,
    hints: &mut StructurePaintHintTable,
    remap_owner: &dyn Fn(&Palette, &str) -> Palette,
    mode: StructureAnimMode,
) -> (usize, usize) {
    let (paint_body, clock_ms) = match mode {
        StructureAnimMode::BodyOnly => (true, None),
        StructureAnimMode::BodyAndAnims { clock_ms } => (true, Some(clock_ms)),
    };
    paint_map_structures_inner(source, map, image, art_rules, hints, remap_owner, paint_body, clock_ms)
}

/// 收集建筑活动层并预解码全部循环帧（不含主体；含黄血燃烧）。
pub fn collect_structure_anim_bank(
    source: &dyn AssetSource,
    map: &MapInfo,
    art_rules: &crate::ArtRules,
    hints: &mut StructurePaintHintTable,
    remap_owner: &dyn Fn(&Palette, &str) -> Palette,
) -> StructureAnimBank {
    let structures: Vec<_> = map.entities.iter().filter(|e| e.kind == MapEntityKind::Structure).collect();
    if structures.is_empty() {
        return StructureAnimBank::default();
    }

    let z_lookup: HashMap<(u16, u16), u8> =
        map.cells.iter().filter(|c| c.x >= 0 && c.y >= 0).map(|c| ((c.x as u16, c.y as u16), c.z)).collect();

    let art = art_rules.art.as_ref();
    collect_structure_type_paint_hints(hints, art_rules, &structures);
    let damage = &art_rules.damage;
    let Some(obj_pal) = load_object_palette(source, map)
    else {
        return StructureAnimBank::default();
    };
    let fire_pal = load_anim_palette(source).unwrap_or_else(|| obj_pal.clone());

    let mut shp_cache: HashMap<String, ShpFile> = HashMap::new();
    let mut anim_hints: HashMap<String, StructureAnimSectionHints> = HashMap::new();
    let mut layers = Vec::new();

    for ent in structures {
        let Some(type_hint) = hints.get(&ent.type_id)
        else {
            continue;
        };
        let remapable = type_hint.remapable;
        let cell_z = z_lookup.get(&(ent.x, ent.y)).copied().unwrap_or(0);
        let yellow = damage.is_yellow(ent.health);

        for (slot, &(_, _, z_key)) in type_hint.loop_anims.iter().zip(STRUCTURE_LOOP_ANIM_KEYS.iter()) {
            let Some(anim_name) = pick_structure_loop_anim_name(slot, yellow)
            else {
                continue;
            };
            // `*ZAdjust` 是原版 Z 缓冲排序偏移，不是屏幕像素。预览叠画已分主体/活动两遍，忽略即可。
            let _ = z_key;
            let hint = cached_anim_section_hint(&mut anim_hints, art, anim_name, 300).clone();
            let anim_remapable = hint.remapable_override.unwrap_or(remapable);
            let anim_pal = if anim_remapable { remap_owner(&obj_pal, &ent.owner) } else { obj_pal.clone() };

            let Some(shp) = load_shp(source, map, &hint.image_key, hint.new_theater, &mut shp_cache)
            else {
                continue;
            };
            let body_n = shp_body_frame_count(&shp.frames) as u16;
            let end = {
                let raw = if hint.loop_end > hint.loop_start {
                    hint.loop_end
                } else {
                    hint.loop_start.saturating_add(1)
                };
                raw.min(body_n.max(hint.loop_start.saturating_add(1)))
            };
            if end <= hint.loop_start {
                continue;
            }
            let mut frames = Vec::with_capacity(usize::from(end.saturating_sub(hint.loop_start)));
            for frame_idx in hint.loop_start..end {
                let Some(blit) = frame_to_blit(shp, frame_idx, 0, &anim_pal)
                else {
                    // 空帧占位，保持下标对齐。
                    frames.push(TileBlit { width: 0, height: 0, offset_x: 0, offset_y: 0, rgba: Vec::new(), shadow: None });
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
                rate_ms: hint.rate_ms,
                loop_start: hint.loop_start,
                loop_end: end,
                frames,
            });
        }

        // 黄血及以下：按 art `DamageFireOffset*` 叠 `DamageFireTypes` 火焰。
        if !yellow || damage.fire_types.is_empty() || type_hint.fire_offsets.is_empty() {
            continue;
        }
        for &(i, ox, oy) in &type_hint.fire_offsets {
            let fire_name = &damage.fire_types[usize::from(i) % damage.fire_types.len()];
            let fire_hint = cached_anim_section_hint(&mut anim_hints, art, fire_name, 80).clone();
            let Some(shp) = load_shp(source, map, &fire_hint.image_key, fire_hint.new_theater, &mut shp_cache)
            else {
                // 无节时仍尝试直接按类型名读 SHP。
                let Some(shp) = load_shp(source, map, fire_name, false, &mut shp_cache)
                else {
                    continue;
                };
                let body_n = shp_body_frame_count(&shp.frames) as u16;
                if body_n == 0 {
                    continue;
                }
                let mut frames = Vec::with_capacity(usize::from(body_n));
                for frame_idx in 0..body_n {
                    let Some(mut blit) = frame_to_blit(shp, frame_idx, 0, &fire_pal)
                    else {
                        frames.push(TileBlit { width: 0, height: 0, offset_x: ox, offset_y: oy, rgba: Vec::new(), shadow: None });
                        continue;
                    };
                    blit.offset_x += ox;
                    blit.offset_y += oy;
                    frames.push(blit);
                }
                if frames.iter().all(|f| f.width == 0) {
                    continue;
                }
                layers.push(StructureAnimLayer {
                    x: ent.x,
                    y: ent.y,
                    cell_z,
                    rate_ms: fire_hint.rate_ms,
                    loop_start: 0,
                    loop_end: body_n,
                    frames,
                });
                continue;
            };
            let body_n = shp_body_frame_count(&shp.frames) as u16;
            if body_n == 0 {
                continue;
            }
            let mut frames = Vec::with_capacity(usize::from(body_n));
            for frame_idx in 0..body_n {
                let Some(mut blit) = frame_to_blit(shp, frame_idx, 0, &fire_pal)
                else {
                    frames.push(TileBlit { width: 0, height: 0, offset_x: ox, offset_y: oy, rgba: Vec::new(), shadow: None });
                    continue;
                };
                blit.offset_x += ox;
                blit.offset_y += oy;
                frames.push(blit);
            }
            if frames.iter().all(|f| f.width == 0) {
                continue;
            }
            layers.push(StructureAnimLayer {
                x: ent.x,
                y: ent.y,
                cell_z,
                rate_ms: fire_hint.rate_ms,
                loop_start: 0,
                loop_end: body_n,
                frames,
            });
        }
    }

    StructureAnimBank { lighting: map.active_lighting(), point_lights: map.point_lights.clone(), layers }
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
        let mut painted = blit.clone();
        apply_rgba_tint(&mut painted.rgba, cell_tint_with_lights(&bank.lighting, layer.cell_z, layer.x, layer.y, &bank.point_lights));
        items.push((layer.x, layer.y, painted));
    }
    let z_at = |x: u16, y: u16| bank.layers.iter().find(|l| l.x == x && l.y == y).map(|l| l.cell_z).unwrap_or(0);
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
    if idx >= frame_count { None } else { Some(idx) }
}

/// 从 art `Buildup=` 装入一次性展开序列。无 `Buildup` 或资源缺失时返回 `None`。
pub fn load_structure_buildup_clip(
    source: &dyn AssetSource,
    map: &MapInfo,
    art_rules: &crate::ArtRules,
    hints: &mut StructurePaintHintTable,
    type_id: &str,
    owner: &str,
    x: u16,
    y: u16,
    remap_owner: &dyn Fn(&Palette, &str) -> Palette,
) -> Option<StructureBuildupClip> {
    let _ = art_rules.art.as_ref()?;
    let techno = TechnoName::parse(type_id);
    hints.ensure(art_rules, &techno);
    let type_hint = hints.get(&techno)?;
    let buildup = type_hint.buildup.as_ref()?;
    let remapable = type_hint.remapable;
    let obj_pal = load_object_palette(source, map)?;
    let pal = if remapable { remap_owner(&obj_pal, owner) } else { obj_pal };
    let mut shp_cache: HashMap<String, ShpFile> = HashMap::new();
    let shp = load_shp(source, map, &buildup.image_key, buildup.new_theater, &mut shp_cache)?;
    // 偶数帧且后半有像素时，后半是落影（常为索引 1）；Buildup 只播主体半幅。
    let body_n = shp_body_frame_count(&shp.frames);
    let mut frames = Vec::with_capacity(body_n);
    for i in 0..body_n {
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
    let cell_z = map.cells.iter().find(|c| c.x == x as i16 && c.y == y as i16).map(|c| c.z).unwrap_or(0);
    Some(StructureBuildupClip { x, y, cell_z, rate_ms: buildup.rate_ms, frames })
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
    art_rules: &crate::ArtRules,
    hints: &mut StructurePaintHintTable,
    remap_owner: &dyn Fn(&Palette, &str) -> Palette,
) -> usize {
    let mut terrain = TerrainImage { image: std::mem::take(image), drawn: 0, origin_x, origin_y };
    let (n, _) = paint_map_structures(source, map, &mut terrain, art_rules, hints, remap_owner, StructureAnimMode::BodyOnly);
    *image = terrain.image;
    n
}

fn paint_map_structures_inner(
    source: &dyn AssetSource,
    map: &MapInfo,
    image: &mut TerrainImage,
    art_rules: &crate::ArtRules,
    hints: &mut StructurePaintHintTable,
    remap_owner: &dyn Fn(&Palette, &str) -> Palette,
    paint_body: bool,
    anim_clock_ms: Option<u64>,
) -> (usize, usize) {
    let structures: Vec<_> = map.entities.iter().filter(|e| e.kind == MapEntityKind::Structure).collect();
    if structures.is_empty() {
        return (0, 0);
    }

    let z_lookup: HashMap<(u16, u16), u8> =
        map.cells.iter().filter(|c| c.x >= 0 && c.y >= 0).map(|c| ((c.x as u16, c.y as u16), c.z)).collect();
    let z_at = |x: u16, y: u16| z_lookup.get(&(x, y)).copied().unwrap_or(0);

    let art = art_rules.art.as_ref();
    collect_structure_type_paint_hints(hints, art_rules, &structures);
    let damage = &art_rules.damage;
    let Some(obj_pal) = load_object_palette(source, map)
    else {
        if !paint_body {
            return (0, 0);
        }
        let missing: Vec<(u16, u16)> = structures.iter().map(|e| (e.x, e.y)).collect();
        let mark = crate::paint_structure_missing_markers(image, &missing, z_at);
        return (0, mark);
    };

    let mut shp_cache: HashMap<String, ShpFile> = HashMap::new();
    let mut blit_cache: HashMap<(String, String, u16, i32), TileBlit> = HashMap::new();
    let mut anim_hints: HashMap<String, StructureAnimSectionHints> = HashMap::new();
    let mut items: Vec<(u16, u16, TileBlit)> = Vec::new();
    let mut missing: Vec<(u16, u16)> = Vec::new();

    for ent in structures {
        let Some(hint) = hints.get(&ent.type_id)
        else {
            continue;
        };
        let remapable = hint.remapable;
        let pal = if remapable { remap_owner(&obj_pal, &ent.owner) } else { obj_pal.clone() };

        if paint_body {
            let body_new_theater = hint.body_new_theater;
            // Bib 垫在主体下（同格、帧 0）；科技前哨等靠它补齐地基。
            if let Some(bib_key) = hint.bib_key.as_ref() {
                if let Some(mut blit) =
                    load_structure_blit(source, map, bib_key, hint.bib_new_theater, 0, 0, &pal, &mut shp_cache, &mut blit_cache, &ent.owner)
                {
                    apply_rgba_tint(&mut blit.rgba, map.tint_at(ent.x, ent.y, z_at(ent.x, ent.y)));
                    items.push((ent.x, ent.y, blit));
                }
            }
            let body_key = hint.body_key.as_str();
            let body_frames =
                load_shp(source, map, body_key, body_new_theater, &mut shp_cache).map(|shp| shp_body_frame_count(&shp.frames)).unwrap_or(1);
            let frame_idx = damaged_body_frame(ent.health, damage.yellow, damage.red, hint.tech_level, body_frames);
            if let Some(mut blit) =
                load_structure_blit(source, map, body_key, body_new_theater, frame_idx, 0, &pal, &mut shp_cache, &mut blit_cache, &ent.owner)
            {
                apply_rgba_tint(&mut blit.rgba, map.tint_at(ent.x, ent.y, z_at(ent.x, ent.y)));
                items.push((ent.x, ent.y, blit));
            } else {
                missing.push((ent.x, ent.y));
            }
            if let Some(mut blit) = load_structure_turret_vxl(source, hint.turret_voxel.as_ref(), ent.facing, &pal) {
                apply_rgba_tint(&mut blit.rgba, map.tint_at(ent.x, ent.y, z_at(ent.x, ent.y)));
                items.push((ent.x, ent.y, blit));
            }
        }

        let Some(clock_ms) = anim_clock_ms
        else {
            continue;
        };
        let yellow = damage.is_yellow(ent.health);
        for (slot, &(_, _, z_key)) in hint.loop_anims.iter().zip(STRUCTURE_LOOP_ANIM_KEYS.iter()) {
            let Some(anim_name) = pick_structure_loop_anim_name(slot, yellow)
            else {
                continue;
            };
            // `*ZAdjust` 仅影响原版 Z 排序，勿当屏幕 Y 像素（医院 `ActiveAnimZAdjust=-200` 会漂到水上）。
            let _ = z_key;
            let anim_hint = cached_anim_section_hint(&mut anim_hints, art, anim_name, 300).clone();
            let frame_idx = structure_anim_frame(clock_ms, anim_hint.rate_ms, anim_hint.loop_start, anim_hint.loop_end);
            let anim_remapable = anim_hint.remapable_override.unwrap_or(remapable);
            let anim_pal = if anim_remapable { remap_owner(&obj_pal, &ent.owner) } else { obj_pal.clone() };
            if let Some(mut blit) = load_structure_blit(
                source,
                map,
                &anim_hint.image_key,
                anim_hint.new_theater,
                frame_idx,
                0,
                &anim_pal,
                &mut shp_cache,
                &mut blit_cache,
                &ent.owner,
            ) {
                apply_rgba_tint(&mut blit.rgba, map.tint_at(ent.x, ent.y, z_at(ent.x, ent.y)));
                items.push((ent.x, ent.y, blit));
            }
        }
    }

    let shp_n = paint_cell_sprites(image, &items, z_at);
    let mark_n = if missing.is_empty() { 0 } else { crate::paint_structure_missing_markers(image, &missing, z_at) };
    (shp_n, mark_n)
}

fn load_object_palette(source: &dyn AssetSource, map: &MapInfo) -> Option<Palette> {
    source
        .read("unittem.pal")
        .ok()
        .and_then(|b| Palette::parse(&b).ok())
        .or_else(|| source.read(theater_palette(map.theater)).ok().and_then(|b| Palette::parse(&b).ok()))
}

/// 特效 / 燃烧动画调色板（`anim.pal`）；缺失时由调用方回退单位盘。
fn load_anim_palette(source: &dyn AssetSource) -> Option<Palette> {
    source.read("anim.pal").ok().and_then(|b| Palette::parse(&b).ok())
}

fn resolve_art_section(art: Option<&IniDocument>, type_id: &str) -> String {
    art.and_then(|a| {
        let image_key = a
            .section(type_id)
            .and_then(|s| s.deserialize::<StructureBodyArtFields>().ok())
            .and_then(|f| f.image)
            .filter(|n| !n.is_empty())
            .unwrap_or_else(|| ImageName::parse(type_id));
        if a.section(image_key.as_str()).is_some() {
            Some(image_key.as_str().to_string())
        } else if a.section(type_id).is_some() {
            Some(type_id.to_ascii_uppercase())
        } else {
            None
        }
    })
        .unwrap_or_else(|| type_id.to_ascii_uppercase())
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
    // 建筑艺术锚点比其高半格。
    // `z_adjust` 仅用于调用方明确要加的像素位移（如 DamageFireOffset），
    // **不要**传入 art `*ZAdjust`（那是 Z 排序，不是像素）。
    Some(TileBlit {
        width: u32::from(frame.frame_width),
        height: u32::from(frame.frame_height),
        offset_x: i32::from(frame.frame_x as i16) - i32::from(shp.width) / 2 + TILE_WIDTH / 2,
        offset_y: i32::from(frame.frame_y as i16) - i32::from(shp.height) / 2 + z_adjust,
        rgba: frame.to_rgba(pal),
        shadow: None,
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
    // 跳过落影半幅（索引常为 1，会画成纯色剪影）。
    if usize::from(frame_idx) >= shp_body_frame_count(&shp.frames) {
        return None;
    }
    let blit = frame_to_blit(shp, frame_idx, z_adjust, pal)?;
    blit_cache.insert(cache_key, blit.clone());
    Some(blit)
}

/// rules `TurretAnim` 体素炮塔（如科技前哨 `OUTP`）；非体素 / 缺资源时跳过。
fn load_structure_turret_vxl(
    source: &dyn AssetSource,
    turret: Option<&StructureTurretVoxelHints>,
    facing: u8,
    pal: &Palette,
) -> Option<TileBlit> {
    let turret = turret?;
    let stem = turret.stem.as_str();
    let anim_x = turret.anim_x;
    let anim_y = turret.anim_y;
    // `TurretAnimZAdjust` 同 ActiveAnim：原版 Z 排序字段，不计入像素。
    let vpl = source.read("voxels.vpl").ok().and_then(|b| VplFile::parse(&b).ok());
    let body_bytes = source.read(&format!("{stem}.vxl")).ok()?;
    let body = VxlFile::parse(&body_bytes).ok()?;
    let body_hva = source.read(&format!("{stem}.hva")).ok().and_then(|b| HvaFile::parse(&b).ok());
    let layers = [VxlLayerPose { vxl: &body, hva: body_hva.as_ref(), facing, frame: 0 }];
    let sprite = rasterize_vxl_layer_poses(&layers, pal, vpl.as_ref())?;
    // 建筑锚点与主体 SHP 同口径（钻石顶边中点）；再加 rules 像素偏移。
    Some(TileBlit {
        width: sprite.width,
        height: sprite.height,
        offset_x: sprite.offset_x + TILE_WIDTH / 2 + anim_x,
        offset_y: sprite.offset_y + anim_y,
        rgba: sprite.rgba,
        shadow: None,
    })
}
