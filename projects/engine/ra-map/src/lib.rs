//! 地图 / 剧院解析与预览合成。

#![deny(missing_docs)]

mod boot_map;
pub mod compose;
mod fallback_preview;
mod iso_math;
mod iso_pack;
mod land;
pub mod lighting;
pub mod mobile_paint;
mod overlay;
mod overlay_paint;
mod overlay_pass;
mod packed_cell;
mod paint_ini;
mod numbered_pack;
mod pass_grid;
mod placements;
pub mod playfield;
mod preview_pack;
pub mod radiation_light;
pub mod scripting;
mod skirmish_preview;
mod smudge;
pub mod structure_damage;
mod structure_paint;
mod terrain_objects;
mod terrain_paint;
mod terrain_preview;
mod theater;
mod tileset;
mod tmp_pass;
pub mod waypoints;
pub mod weather_particles;

/// Base64 编解码（地图二进制段）。
pub mod base64;
/// LCW / Format80 解压（覆盖层包）。
pub mod lcw;
/// LZO1X 解压（IsoMapPack5）。
pub mod lzo;

use ra_assets::{IniDocument, from_row, numbered_pairs};
use ra_types::{
    GameEdition, MapDefinition, MapIsoCell, MapLighting, MapLocalSize, MapOverlayCell, MapPlacedEntity, MapPlacedEntityKind, MapTerrainObject,
    MapWeatherKind,
    MapWaypoint, RaError, RaResult,
};
use serde::Deserialize;
use serde::de::{self, Deserializer};

pub use base64::{base64_decode, base64_encode};
pub use boot_map::{
    BOOT_MAP_CANDIDATES, BootMapCandidate, BootMapResult, boot_map_name_csf_key, count_skirmish_start_slots, find_boot_map,
    find_boot_map_named, find_first_boot_map, list_parseable_boot_maps, list_parseable_maps_from_missions_pkt, list_parseable_maps_from_names,
    mount_theater_mixes, resolve_boot_map_name_csf, skirmish_ai_row_count, try_parse_boot_map,
};
pub use compose::{
    ShadowBlit, TerrainImage, TileBlit, compose_terrain_rgba, paint_cell_sprites, paint_overlay_markers, paint_structure_missing_markers,
};
pub use fallback_preview::{RawRgbaImage, load_fallback_theater_tile, load_fallback_unit_sprite};
pub use iso_math::{HEIGHT_STEP, TILE_HEIGHT, TILE_WIDTH, iso_to_screen, screen_to_iso};
pub use iso_pack::{IsoCell, decode_iso_map_pack, parse_iso_cells};
pub use land::{LandType, ground_passable, land_passable, tmp_terrain_to_land_type};
pub use lighting::{
    LEPTONS_PER_CELL, LightingConfig, LightingProfile, MapLightingProfiles, PointLight, StructureLightTable, apply_rgba_tint, cell_light_scalar,
    cell_tint, cell_tint_with_lights, collect_structure_point_lights, light_value_to_units, parse_lighting, parse_map_lighting, point_light_at,
    point_light_from_profile, radiation_point_light, terrain_tint,
};
pub use mobile_paint::{MobilePaintPose, infantry_facing_slot, paint_map_mobiles};
pub use overlay::{NO_OVERLAY, OVERLAY_CELLS, OVERLAY_GRID, OverlayCell, decode_overlay_packs};
pub use overlay_paint::{
    OverlayLayerFilter, flat_tiberium_display_type_name, is_bridge_overlay_name, paint_map_overlays, paint_overlays_onto_preview_rgba,
};
pub use overlay_pass::apply_overlay_land_to_pass_grid;
pub use paint_ini::{PaintIniDocs, read_optional_ini};
pub use pass_grid::{MAX_GROUND_CLIMB, PassGrid};
pub use placements::{MapEntity, MapEntityKind, parse_map_entities};
pub use playfield::{LocalSize, cell_in_local_playfield, local_size_preview_rect};
pub use preview_pack::{
    MapPreviewImage, decode_preview_from_ini, decode_preview_from_map_bytes, decode_preview_pack, decode_preview_pack_bytes, parse_preview_size,
};
pub use radiation_light::{
    RadiationLightRules, RadiationLightSite, collect_radiation_lights, parse_radiation_light_rules, radiation_light_epoch,
    radiation_site_light, radiation_site_radius_leptons,
};
pub use scripting::{
    MapAction, MapActionCommand, MapActionKind, MapAiTrigger, MapCapabilityGap, MapCellTag, MapEvent, MapEventCondition, MapEventKind,
    MapHouse, MapScriptStep, MapScriptType, MapScripting, MapTag, MapTaskForce, MapTaskForceEntry, MapTeamType, MapTrigger,
    campaign_blocking_capability_message, map_scripting_capability_gaps, parse_map_houses, parse_map_scripting,
};
pub use skirmish_preview::{
    BootPreviewResult, SkirmishPreviewStats, compose_boot_preview, compose_skirmish_preview, paint_mobiles_onto_preview_rgba,
};
pub use smudge::{MapSmudge, parse_map_smudges};
pub use structure_damage::{
    StructureDamageRules, damaged_body_frame, health_ratio_256, parse_condition_percent, parse_damage_fire_offset, structure_tech_level,
};
pub use structure_paint::{
    StructureAnimBank, StructureAnimLayer, StructureAnimMode, StructureBuildupClip, buildup_frame_index, collect_structure_anim_bank,
    load_structure_buildup_clip, paint_map_structures, paint_structure_anim_bank, paint_structure_anims_onto_rgba,
    paint_structure_buildup_onto_rgba, paint_structures_onto_rgba, structure_anim_frame,
};
pub use terrain_objects::{TerrainObject, parse_terrain_objects};
pub use terrain_paint::{
    TerrainAnimBank, TerrainAnimLayer, TerrainPaintMode, collect_ore_tree_anim_bank, collect_terrain_anim_bank, format_terrain_anim_layer_diag,
    ore_tree_frame_count_hints, paint_map_terrain_objects, paint_ore_tree_frames, paint_ore_tree_frames_onto_rgba, paint_terrain_anim_bank,
    paint_terrain_anims_onto_rgba, terrain_anim_frame, terrain_animation_rate_ms,
};
pub use terrain_preview::compose_terrain_preview;
pub use theater::{
    Theater, new_theater_shp_name, theater_ini_name, theater_mix_names, theater_new_letter, theater_palette, theater_tiberium_palette,
    theater_tmp_extension,
};
pub use tileset::{CLEAR_TILE_SENTINEL, TilesetLookup, normalize_tile_ref, parse_tileset_ini};
pub use tmp_pass::seal_pass_grid_from_tmp;
pub use waypoints::{Waypoint, parse_waypoints, skirmish_start_waypoint};
pub use weather_particles::{WeatherKind, WeatherParticleField};

/// 地图基本信息（可附带已解码的 IsoMapPack / Overlay / Terrain / 放置 / 航点）。
#[derive(Debug, Clone)]
pub struct MapInfo {
    /// 游戏版本。
    pub edition: GameEdition,
    /// 地图名（通常为文件名）。
    pub name: String,
    /// `[Map] Size` 宽（菱形参数，不是通行格网宽）。
    pub size_width: u32,
    /// `[Map] Size` 高（菱形参数，不是通行格网高）。
    pub size_height: u32,
    /// `[Map] LocalSize`：镜头可见内缘（缺省等于整张 `Size`）。
    pub local_size: LocalSize,
    /// 游戏格网宽（与 iso / 航点 / 覆盖层同一坐标系）。
    pub width: u32,
    /// 游戏格网高（与 iso / 航点 / 覆盖层同一坐标系）。
    pub height: u32,
    /// 剧院。
    pub theater: Theater,
    /// `[Basic] GameModes` 标签（逗号分隔解析；空表示仅匹配 `standard`）。
    pub game_modes: Vec<String>,
    /// `[Basic] Description` CSF 键（可空；官方遭遇图常省略）。
    pub description_csf: String,
    /// `[Basic] NextMission`：战役胜利后下一关地图文件名（可空）。
    pub next_mission: String,
    /// `[Basic] AlternateNextMission`：战役失败后下一关 / 分支地图文件名（可空）。
    pub alternate_next_mission: String,
    /// `[Basic] StartingCredits`：开局资金；`0` 表示节内未写或显式为 0。
    pub starting_credits: i32,
    /// `[Lighting]` 全局环境光（缺节用零售缺省，含 `Ground=0.20`）。
    pub lighting: LightingConfig,
    /// `[Lighting]` Ion / 闪电风暴档（缺键用零售 Ion 缺省）。
    pub ion_lighting: LightingConfig,
    /// 当前生效的环境光档（缺省普通；风暴切换时设为 `Ion`）。
    pub lighting_profile: LightingProfile,
    /// 建筑点光源（rules `LightIntensity`；由 [`Self::refresh_point_lights`] 刷新）。
    pub structure_point_lights: Vec<PointLight>,
    /// 辐射站点绿光（由 [`Self::refresh_radiation_lights`] 刷新）。
    pub radiation_point_lights: Vec<PointLight>,
    /// 叠画用合并点光源（建筑 + 辐射）。
    pub point_lights: Vec<PointLight>,
    /// 等距地形单元。
    pub cells: Vec<IsoCell>,
    /// 覆盖层格。
    pub overlays: Vec<OverlayCell>,
    /// 静态地形物件。
    pub terrain_objects: Vec<TerrainObject>,
    /// `[Smudge]` 污迹占位。
    pub smudges: Vec<MapSmudge>,
    /// 预放实体。
    pub entities: Vec<MapEntity>,
    /// 航点。
    pub waypoints: Vec<Waypoint>,
    /// 剧本节（Triggers / Teams / Houses 等）。
    pub scripting: MapScripting,
    /// `[Preview] Size` 宽（缺节或无效为 0；不含像素载荷）。
    pub preview_width: u32,
    /// `[Preview] Size` 高（缺节或无效为 0；不含像素载荷）。
    pub preview_height: u32,
    /// `[Digest]` 编号键按序拼接的校验摘要（缺节为空）。
    pub digest: String,
}

impl MapInfo {
    /// 空地图占位（尺寸为 0，剧院为温带）。
    pub fn empty(edition: GameEdition, name: impl Into<String>) -> Self {
        Self {
            edition,
            name: name.into(),
            size_width: 0,
            size_height: 0,
            local_size: LocalSize::from_full_size(1, 1),
            width: 0,
            height: 0,
            theater: Theater::Temperate,
            game_modes: Vec::new(),
            description_csf: String::new(),
            next_mission: String::new(),
            alternate_next_mission: String::new(),
            starting_credits: 0,
            lighting: LightingConfig::default(),
            ion_lighting: LightingConfig::ion_default(),
            lighting_profile: LightingProfile::Normal,
            structure_point_lights: Vec::new(),
            radiation_point_lights: Vec::new(),
            point_lights: Vec::new(),
            cells: Vec::new(),
            overlays: Vec::new(),
            terrain_objects: Vec::new(),
            smudges: Vec::new(),
            entities: Vec::new(),
            waypoints: Vec::new(),
            scripting: MapScripting::default(),
            preview_width: 0,
            preview_height: 0,
            digest: String::new(),
        }
    }

    /// 从场景 INI（`.map` / `.mpr`）解析尺寸、剧院，并尝试解码地形与覆盖层。
    pub fn parse_ini(edition: GameEdition, name: impl Into<String>, bytes: &[u8]) -> RaResult<Self> {
        let doc = IniDocument::parse(bytes)?;
        let map_fields = match doc.section("Map") {
            Some(sec) => sec
                .deserialize::<MapSectionFields>()
                .map_err(|e| RaError::Parse(format!("[Map] 节无效: {e}")))?,
            None => MapSectionFields::default(),
        };
        let (size_width, size_height) = map_fields
            .size
            .ok_or_else(|| RaError::Parse("地图缺少 [Map] Size".into()))?;
        let local_size = map_fields
            .local_size
            .unwrap_or_else(|| LocalSize::from_full_size(size_width, size_height));
        // 航点 / IsoMapPack / 覆盖层落在方形游戏格空间，边长为 Size 高 + max(宽, 高)。
        let side = game_cell_grid_side(size_width, size_height);
        let theater_raw = map_fields.theater.as_deref().unwrap_or("TEMPERATE");
        let theater = Theater::parse(theater_raw)?;
        let basic = doc
            .section("Basic")
            .and_then(|s| s.deserialize::<BasicSectionFields>().ok())
            .unwrap_or_default();
        let game_modes = basic.game_modes;
        let description_csf = basic.description.unwrap_or_default().trim().to_string();
        let next_mission = basic.next_mission.unwrap_or_default().trim().to_string();
        let alternate_next_mission = basic.alternate_next_mission.unwrap_or_default().trim().to_string();
        let starting_credits = basic.starting_credits.unwrap_or(0).max(0);
        let profiles = parse_map_lighting(&doc);
        let cells = match decode_iso_map_pack(&doc) {
            Ok(c) => c,
            Err(_) => Vec::new(),
        };
        let overlays = match decode_overlay_packs(&doc) {
            Ok(o) => o,
            Err(_) => Vec::new(),
        };
        let terrain_objects = parse_terrain_objects(&doc);
        let smudges = parse_map_smudges(&doc);
        let entities = parse_map_entities(&doc);
        let waypoints = parse_waypoints(&doc);
        let scripting = parse_map_scripting(&doc);
        let (preview_width, preview_height) = doc
            .section("Preview")
            .and_then(|s| s.deserialize::<crate::preview_pack::PreviewSectionFields>().ok())
            .and_then(|f| f.size)
            .unwrap_or((0, 0));
        let digest = parse_map_digest(&doc);
        Ok(Self {
            edition,
            name: name.into(),
            size_width,
            size_height,
            local_size,
            width: side,
            height: side,
            theater,
            game_modes,
            description_csf,
            next_mission,
            alternate_next_mission,
            starting_credits,
            lighting: profiles.normal,
            ion_lighting: profiles.ion,
            lighting_profile: LightingProfile::Normal,
            structure_point_lights: Vec::new(),
            radiation_point_lights: Vec::new(),
            point_lights: Vec::new(),
            cells,
            overlays,
            terrain_objects,
            smudges,
            entities,
            waypoints,
            scripting,
            preview_width,
            preview_height,
            digest,
        })
    }

    /// 战役结算后续关 scenario：胜用 `NextMission`，败用 `AlternateNextMission`；空则 `None`。
    pub fn campaign_continue_scenario(&self, victory: bool) -> Option<&str> {
        let raw = if victory { self.next_mission.as_str() } else { self.alternate_next_mission.as_str() };
        let trimmed = raw.trim();
        if trimmed.is_empty() { None } else { Some(trimmed) }
    }

    /// 提取冻结 [`MapDefinition`] 骨架（含触发链 / 队伍脚本 / AI / Preview 尺寸 / Digest / Smudge / 天气种类；不含预览像素与粒子场）。
    pub fn to_map_definition(&self) -> MapDefinition {
        MapDefinition {
            name: self.name.clone(),
            size_width: self.size_width,
            size_height: self.size_height,
            local_size: MapLocalSize {
                left: self.local_size.left,
                top: self.local_size.top,
                width: self.local_size.width,
                height: self.local_size.height,
            },
            cell_side: self.width,
            theater: self.theater.as_str().to_ascii_uppercase(),
            description_csf: self.description_csf.clone(),
            game_modes: self.game_modes.clone(),
            next_mission: self.next_mission.clone(),
            alternate_next_mission: self.alternate_next_mission.clone(),
            starting_credits: self.starting_credits,
            lighting: map_lighting_from_config(&self.lighting),
            ion_lighting: map_lighting_from_config(&self.ion_lighting),
            waypoints: self
                .waypoints
                .iter()
                .map(|w| MapWaypoint { index: w.index, x: w.x, y: w.y })
                .collect(),
            terrain_objects: self
                .terrain_objects
                .iter()
                .map(|t| MapTerrainObject { x: t.x, y: t.y, name: t.name.clone() })
                .collect(),
            smudges: self
                .smudges
                .iter()
                .map(|s| ra_types::MapSmudge { x: s.x, y: s.y, name: s.name.clone() })
                .collect(),
            entities: self.entities.iter().map(map_entity_to_placed).collect(),
            cells: self
                .cells
                .iter()
                .map(|c| MapIsoCell {
                    x: c.x,
                    y: c.y,
                    tile_num: c.tile_num,
                    sub_tile: c.sub_tile,
                    z: c.z,
                    flags: c.flags,
                })
                .collect(),
            overlays: self
                .overlays
                .iter()
                .map(|o| MapOverlayCell {
                    x: o.x,
                    y: o.y,
                    overlay_id: o.overlay_id,
                    data: o.data,
                })
                .collect(),
            houses: self.scripting.houses.iter().map(map_house_to_definition).collect(),
            tags: self.scripting.tags.iter().map(map_tag_to_definition).collect(),
            triggers: self.scripting.triggers.iter().map(map_trigger_to_definition).collect(),
            events: self.scripting.events.iter().map(map_event_to_definition).collect(),
            actions: self.scripting.actions.iter().map(map_action_to_definition).collect(),
            cell_tags: self.scripting.cell_tags.iter().map(map_cell_tag_to_definition).collect(),
            task_forces: self.scripting.task_forces.iter().map(map_task_force_to_definition).collect(),
            script_types: self.scripting.script_types.iter().map(map_script_type_to_definition).collect(),
            team_types: self.scripting.team_types.iter().map(map_team_type_to_definition).collect(),
            ai_triggers: self.scripting.ai_triggers.iter().map(map_ai_trigger_to_definition).collect(),
            preview_width: self.preview_width,
            preview_height: self.preview_height,
            digest: self.digest.clone(),
            weather: map_weather_kind_from_theater(self.theater),
        }
    }

    /// 提取 [`ra_types::PreparedMap`] 骨架：冻结定义 + 由 [`PassGrid::from_map`] 灌入的通行层 + 锚点粗占格。
    ///
    /// 不含 overlay 陆地封格、Foundation 展开、渲染清单或规则绑定；对局路径仍可继续用 `PassGrid` 直至准备层收口。
    pub fn to_prepared_map_skeleton(&self) -> ra_types::PreparedMap {
        self.prepared_map_from_pass_grid(PassGrid::from_map(self), None)
    }

    /// 在 [`Self::to_prepared_map_skeleton`] 基础上，用 overlay `Land=` / `NoUseTileLandType` 覆写通行层。
    ///
    /// 仍不含 Foundation 展开或渲染清单；调用方需已装载 [`ra_types::OverlayTypeRegistry`]。
    pub fn to_prepared_map_skeleton_with_overlays(&self, overlays: &ra_types::OverlayTypeRegistry) -> ra_types::PreparedMap {
        let mut grid = PassGrid::from_map(self);
        apply_overlay_land_to_pass_grid(self, overlays, &mut grid);
        self.prepared_map_from_pass_grid(grid, None)
    }

    /// 在 [`Self::to_prepared_map_skeleton`] 基础上，按建筑表 `Foundation=` 展开 occupancy 与通行封格。
    ///
    /// 未知类型回退 `1x1`；仍不含 overlay 陆地覆写或渲染清单。
    pub fn to_prepared_map_skeleton_with_structures(&self, structures: &ra_types::StructureDefinitions) -> ra_types::PreparedMap {
        self.prepared_map_from_pass_grid(PassGrid::from_map_with_structures(self, Some(structures)), Some(structures))
    }

    /// 绑定 overlay 通行覆写与建筑 `Foundation=` 的准备骨架（产品装载主入口候选）。
    ///
    /// 仍不含渲染清单或 name→稳定 id 绑定。
    pub fn to_prepared_map_skeleton_bound(
        &self,
        overlays: &ra_types::OverlayTypeRegistry,
        structures: &ra_types::StructureDefinitions,
    ) -> ra_types::PreparedMap {
        let mut grid = PassGrid::from_map_with_structures(self, Some(structures));
        apply_overlay_land_to_pass_grid(self, overlays, &mut grid);
        self.prepared_map_from_pass_grid(grid, Some(structures))
    }

    fn prepared_map_from_pass_grid(
        &self,
        grid: PassGrid,
        structures: Option<&ra_types::StructureDefinitions>,
    ) -> ra_types::PreparedMap {
        let definition = self.to_map_definition();
        let (pass_width, pass_height, passable, cell_heights) = grid.to_prepared_pass_layers();
        let occupancy = prepared_occupancy_from_map(self, structures);
        ra_types::PreparedMap {
            definition,
            pass_width,
            pass_height,
            passable,
            cell_heights,
            occupancy,
        }
    }

    /// 当前档的环境光配置。
    pub fn active_lighting(&self) -> LightingConfig {
        match self.lighting_profile {
            LightingProfile::Normal => self.lighting,
            LightingProfile::Ion => self.ion_lighting,
        }
    }

    /// 切换普通 / Ion 环境光档（闪电风暴等）。
    pub fn set_lighting_profile(&mut self, profile: LightingProfile) {
        self.lighting_profile = profile;
    }

    /// 用建筑光表刷新点光源，并与辐射光合并进 `point_lights`。
    pub fn refresh_point_lights(&mut self, lights: &crate::lighting::StructureLightTable) {
        self.structure_point_lights = collect_structure_point_lights(&self.entities, lights);
        self.rebuild_point_lights();
    }

    /// 用当前辐射站点快照刷新绿光，并与建筑光合并进 `point_lights`。
    pub fn refresh_radiation_lights(&mut self, sites: &[RadiationLightSite], rules: &RadiationLightRules) {
        self.radiation_point_lights = collect_radiation_lights(sites, rules);
        self.rebuild_point_lights();
    }

    fn rebuild_point_lights(&mut self) {
        self.point_lights.clear();
        self.point_lights.reserve(self.structure_point_lights.len() + self.radiation_point_lights.len());
        self.point_lights.extend_from_slice(&self.structure_point_lights);
        self.point_lights.extend_from_slice(&self.radiation_point_lights);
    }

    /// 当前档环境光 + 点光源的格 tint（地形砖请传 `z=0` 以免接缝）。
    pub fn tint_at(&self, x: u16, y: u16, z: u8) -> [f32; 3] {
        cell_tint_with_lights(&self.active_lighting(), z, x, y, &self.point_lights)
    }
}

/// 解析逗号分隔的游戏模式标签（去空白、丢空段）。
///
/// 地图 `[Basic] GameModes` 已由节 Serde 直接落到 `Vec`；本函数供 missions.pkt 等外层字符串入口复用。
pub fn parse_game_modes(raw: Option<&str>) -> Vec<String> {
    let Some(raw) = raw
    else {
        return Vec::new();
    };
    raw.split(',').map(str::trim).filter(|s| !s.is_empty()).map(str::to_string).collect()
}

/// 地图是否匹配模式表中的 `map_filter`。
///
/// 空 `game_modes` 只接受过滤标签 `standard`（大小写不敏感）。
pub fn map_matches_game_mode_filter(game_modes: &[String], filter: &str) -> bool {
    let filter = filter.trim();
    if filter.is_empty() {
        return false;
    }
    if game_modes.is_empty() {
        return filter.eq_ignore_ascii_case("standard");
    }
    game_modes.iter().any(|m| m.eq_ignore_ascii_case(filter))
}

/// 由 `[Map] Size` 宽高得到方形游戏格网边长。
///
/// Iso 坐标与航点落在同一空间；iso X/Y 上界约为 `Width+Height`，边长取
/// `height + max(width, height)`，保证菱形外包完整且不小于常见的 `2*height` 垫法。
pub fn game_cell_grid_side(size_width: u32, size_height: u32) -> u32 {
    size_height.saturating_add(size_width.max(size_height))
}

/// `[Map]` 节字段（一次 Serde；`Size` / `LocalSize` 在反序列化时解码为结构化宽高）。
#[derive(Debug, Default, Deserialize)]
struct MapSectionFields {
    #[serde(rename = "Size", default, deserialize_with = "de_opt_map_size")]
    size: Option<(u32, u32)>,
    #[serde(rename = "LocalSize", default, deserialize_with = "de_opt_local_size")]
    local_size: Option<LocalSize>,
    #[serde(rename = "Theater")]
    theater: Option<String>,
}

/// `[Basic]` 节字段（一次 Serde）。
#[derive(Debug, Default, Deserialize)]
struct BasicSectionFields {
    #[serde(rename = "GameModes", default)]
    game_modes: Vec<String>,
    #[serde(rename = "Description")]
    description: Option<String>,
    #[serde(rename = "NextMission")]
    next_mission: Option<String>,
    #[serde(rename = "AlternateNextMission")]
    alternate_next_mission: Option<String>,
    #[serde(rename = "StartingCredits")]
    starting_credits: Option<i32>,
}

/// `[Digest]` 编号键按序拼接；缺节或全空为 `""`。
fn parse_map_digest(doc: &IniDocument) -> String {
    let Some(section) = doc.section("Digest") else {
        return String::new();
    };
    numbered_pairs(section)
        .into_iter()
        .map(|(_, value)| value.trim())
        .filter(|value| !value.is_empty())
        .collect::<Vec<_>>()
        .join("")
}

/// 建筑占地=`1`、地形物件=`2`、污迹=`3`；同格建筑优先。尺寸与 [`MapInfo::width`] / [`MapInfo::height`] 对齐。
///
/// 若提供 `structures`，按 `Foundation=` 从锚点向右下展开；缺表或未知类型按 `1x1`。
fn prepared_occupancy_from_map(map: &MapInfo, structures: Option<&ra_types::StructureDefinitions>) -> Vec<u8> {
    let width = map.width.max(1) as usize;
    let height = map.height.max(1) as usize;
    let mut occupancy = vec![0u8; width.saturating_mul(height)];
    let mark = |occ: &mut [u8], x: u16, y: u16, kind: u8| {
        let xi = usize::from(x);
        let yi = usize::from(y);
        if xi >= width || yi >= height {
            return;
        }
        let i = yi * width + xi;
        if occ[i] == 0 || kind == 1 {
            occ[i] = kind;
        }
    };
    for ent in &map.entities {
        if ent.kind != MapEntityKind::Structure {
            continue;
        }
        let (fw, fh) = structures
            .and_then(|table| table.get(&ent.type_id))
            .map(|def| (def.foundation.width.max(1), def.foundation.height.max(1)))
            .unwrap_or((1, 1));
        for dy in 0..fh {
            for dx in 0..fw {
                mark(
                    &mut occupancy,
                    ent.x.saturating_add(dx),
                    ent.y.saturating_add(dy),
                    1,
                );
            }
        }
    }
    for obj in &map.terrain_objects {
        mark(&mut occupancy, obj.x, obj.y, 2);
    }
    for smudge in &map.smudges {
        mark(&mut occupancy, smudge.x, smudge.y, 3);
    }
    occupancy
}

/// `Size=x,y,width,height` 行（前两列原点，后两列宽高）。
#[derive(Debug, Deserialize)]
struct MapSizeRow {
    _origin_x: i32,
    _origin_y: i32,
    width: u32,
    height: u32,
}

fn map_entity_to_placed(ent: &MapEntity) -> MapPlacedEntity {
    MapPlacedEntity {
        kind: match ent.kind {
            MapEntityKind::Structure => MapPlacedEntityKind::Structure,
            MapEntityKind::Unit => MapPlacedEntityKind::Unit,
            MapEntityKind::Infantry => MapPlacedEntityKind::Infantry,
            MapEntityKind::Aircraft => MapPlacedEntityKind::Aircraft,
        },
        owner: ent.owner.clone(),
        type_id: ent.type_id.clone(),
        health: ent.health,
        x: ent.x,
        y: ent.y,
        facing: ent.facing,
        sub_cell: ent.sub_cell,
        mission: ent.mission.clone(),
        tag: ent.tag.clone(),
    }
}

fn map_house_to_definition(house: &crate::scripting::MapHouse) -> ra_types::MapHouse {
    ra_types::MapHouse {
        name: house.name.clone(),
        country: house.country.clone(),
        tech_level: house.tech_level,
        credits: house.credits,
        iq: house.iq,
        edge: house.edge.clone(),
        player_control: house.player_control,
        color: house.color.clone(),
        allies: house.allies.clone(),
    }
}

fn map_tag_to_definition(tag: &crate::scripting::MapTag) -> ra_types::MapTag {
    ra_types::MapTag {
        id: tag.id.clone(),
        persistence: tag.persistence,
        name: tag.name.clone(),
        trigger_id: tag.trigger_id.clone(),
    }
}

fn map_trigger_to_definition(trigger: &crate::scripting::MapTrigger) -> ra_types::MapTrigger {
    ra_types::MapTrigger {
        id: trigger.id.clone(),
        house: trigger.house.clone(),
        linked: trigger.linked.clone(),
        name: trigger.name.clone(),
        disabled: trigger.disabled,
        easy: trigger.easy,
        normal: trigger.normal,
        hard: trigger.hard,
    }
}

fn map_event_to_definition(event: &crate::scripting::MapEvent) -> ra_types::MapEvent {
    ra_types::MapEvent {
        id: event.id.clone(),
        conditions: event
            .conditions
            .iter()
            .map(|c| ra_types::MapEventCondition {
                kind_code: c.kind.code(),
                params: c.params.clone(),
            })
            .collect(),
    }
}

fn map_action_to_definition(action: &crate::scripting::MapAction) -> ra_types::MapAction {
    ra_types::MapAction {
        id: action.id.clone(),
        commands: action
            .commands
            .iter()
            .map(|c| ra_types::MapActionCommand {
                kind_code: c.kind.code(),
                params: c.params.clone(),
            })
            .collect(),
    }
}

fn map_cell_tag_to_definition(cell: &crate::scripting::MapCellTag) -> ra_types::MapCellTag {
    ra_types::MapCellTag { x: cell.x, y: cell.y, tag_id: cell.tag_id.clone() }
}

fn map_task_force_to_definition(tf: &crate::scripting::MapTaskForce) -> ra_types::MapTaskForce {
    ra_types::MapTaskForce {
        id: tf.id.clone(),
        name: tf.name.clone(),
        entries: tf
            .entries
            .iter()
            .map(|e| ra_types::MapTaskForceEntry {
                count: e.count,
                type_id: e.type_id.clone(),
            })
            .collect(),
        group: tf.group,
    }
}

fn map_script_type_to_definition(script: &crate::scripting::MapScriptType) -> ra_types::MapScriptType {
    ra_types::MapScriptType {
        id: script.id.clone(),
        name: script.name.clone(),
        steps: script
            .steps
            .iter()
            .map(|s| ra_types::MapScriptStep {
                action: s.action,
                argument: s.argument,
            })
            .collect(),
    }
}

fn map_team_type_to_definition(team: &crate::scripting::MapTeamType) -> ra_types::MapTeamType {
    ra_types::MapTeamType {
        id: team.id.clone(),
        name: team.name.clone(),
        house: team.house.clone(),
        script: team.script.clone(),
        task_force: team.task_force.clone(),
        tag: team.tag.clone(),
        waypoint: team.waypoint,
        max: team.max,
        priority: team.priority,
        veteran_level: team.veteran_level,
    }
}

fn map_ai_trigger_to_definition(trigger: &crate::scripting::MapAiTrigger) -> ra_types::MapAiTrigger {
    ra_types::MapAiTrigger {
        id: trigger.id.clone(),
        name: trigger.name.clone(),
        team: trigger.team.clone(),
        owner_house: trigger.owner_house.clone(),
        tech_level: trigger.tech_level,
    }
}

/// 剧院默认天气种类（与 `WeatherParticleField::for_theater` 对齐）。
fn map_weather_kind_from_theater(theater: Theater) -> MapWeatherKind {
    match theater {
        Theater::Snow => MapWeatherKind::Snow,
        _ => MapWeatherKind::None,
    }
}

fn map_lighting_from_config(cfg: &LightingConfig) -> MapLighting {
    MapLighting {
        ambient: cfg.ambient,
        red: cfg.red,
        green: cfg.green,
        blue: cfg.blue,
        ground: cfg.ground,
        level: cfg.level,
    }
}

/// `LocalSize=left,top,width,height` 行。
#[derive(Debug, Deserialize)]
struct LocalSizeRow {
    left: i32,
    top: i32,
    width: i32,
    height: i32,
}

fn de_opt_map_size<'de, D>(deserializer: D) -> Result<Option<(u32, u32)>, D::Error>
where
    D: Deserializer<'de>,
{
    let raw = String::deserialize(deserializer)?;
    let row: MapSizeRow = from_row(&raw).map_err(de::Error::custom)?;
    Ok(Some((row.width, row.height)))
}

fn de_opt_local_size<'de, D>(deserializer: D) -> Result<Option<LocalSize>, D::Error>
where
    D: Deserializer<'de>,
{
    let raw = String::deserialize(deserializer)?;
    let Ok(row) = from_row::<LocalSizeRow>(&raw)
    else {
        return Ok(None);
    };
    if row.width <= 0 || row.height <= 0 {
        return Ok(None);
    }
    Ok(Some(LocalSize {
        left: row.left,
        top: row.top,
        width: row.width,
        height: row.height,
    }))
}
