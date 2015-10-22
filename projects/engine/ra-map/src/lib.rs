//! 地图 / 剧院解析与预览合成。

#![deny(missing_docs)]

mod boot_map;
mod compose;
mod fallback_preview;
mod iso_math;
mod iso_pack;
mod land;
mod mobile_paint;
mod overlay;
mod overlay_paint;
mod pass_grid;
mod placements;
mod preview_pack;
mod skirmish_preview;
mod structure_damage;
mod structure_paint;
mod terrain_objects;
mod terrain_paint;
mod terrain_preview;
mod theater;
mod tileset;
mod tmp_pass;
mod waypoints;

/// Base64 编解码（地图二进制段）。
pub mod base64;
/// LCW / Format80 解压（覆盖层包）。
pub mod lcw;
/// LZO1X 解压（IsoMapPack5）。
pub mod lzo;

use ra_assets::IniDocument;
use ra_types::{GameEdition, RaError, RaResult};

pub use base64::{base64_decode, base64_encode};
pub use boot_map::{
    BOOT_MAP_CANDIDATES, BootMapCandidate, BootMapResult, boot_map_name_csf_key, count_skirmish_start_slots, find_boot_map,
    find_boot_map_named, find_first_boot_map, list_parseable_boot_maps, list_parseable_maps_from_missions_pkt,
    list_parseable_maps_from_names, mount_theater_mixes, resolve_boot_map_name_csf, skirmish_ai_row_count, try_parse_boot_map,
};
pub use compose::{TerrainImage, TileBlit, compose_terrain_rgba, paint_cell_sprites, paint_overlay_markers};
pub use fallback_preview::{RawRgbaImage, load_fallback_theater_tile, load_fallback_unit_sprite};
pub use iso_math::{HEIGHT_STEP, TILE_HEIGHT, TILE_WIDTH, iso_to_screen, screen_to_iso};
pub use iso_pack::{IsoCell, decode_iso_map_pack, parse_iso_cells};
pub use land::{LandType, ground_passable};
pub use mobile_paint::paint_map_mobiles;
pub use overlay::{NO_OVERLAY, OVERLAY_CELLS, OVERLAY_GRID, OverlayCell, decode_overlay_packs};
pub use overlay_paint::{flat_tiberium_display_type_name, paint_map_overlays};
pub use pass_grid::{MAX_GROUND_CLIMB, PassGrid};
pub use placements::{MapEntity, MapEntityKind, parse_map_entities};
pub use preview_pack::{
    MapPreviewImage, decode_preview_from_ini, decode_preview_from_map_bytes, decode_preview_pack, parse_preview_size,
};
pub use skirmish_preview::{
    BootPreviewResult, SkirmishPreviewStats, compose_boot_preview, compose_skirmish_preview, paint_mobiles_onto_preview_rgba,
};
pub use structure_damage::{
    StructureDamageRules, damaged_body_frame, health_ratio_256, parse_condition_percent, parse_damage_fire_offset,
};
pub use structure_paint::{
    StructureAnimBank, StructureAnimLayer, StructureAnimMode, StructureBuildupClip, buildup_frame_index,
    collect_structure_anim_bank, load_structure_buildup_clip, paint_map_structures, paint_structure_anim_bank,
    paint_structure_anims_onto_rgba, paint_structure_buildup_onto_rgba, paint_structures_onto_rgba, structure_anim_frame,
};
pub use terrain_objects::{TerrainObject, parse_terrain_objects};
pub use terrain_paint::paint_map_terrain_objects;
pub use terrain_preview::compose_terrain_preview;
pub use theater::{
    Theater, new_theater_shp_name, theater_ini_name, theater_mix_names, theater_new_letter, theater_palette, theater_tiberium_palette,
    theater_tmp_extension,
};
pub use tileset::{CLEAR_TILE_SENTINEL, TilesetLookup, normalize_tile_ref, parse_tileset_ini};
pub use tmp_pass::seal_pass_grid_from_tmp;
pub use waypoints::{Waypoint, parse_waypoints, skirmish_start_waypoint};

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
    /// 等距地形单元。
    pub cells: Vec<IsoCell>,
    /// 覆盖层格。
    pub overlays: Vec<OverlayCell>,
    /// 静态地形物件。
    pub terrain_objects: Vec<TerrainObject>,
    /// 预放实体。
    pub entities: Vec<MapEntity>,
    /// 航点。
    pub waypoints: Vec<Waypoint>,
}

impl MapInfo {
    /// 空地图占位（尺寸为 0，剧院为温带）。
    pub fn empty(edition: GameEdition, name: impl Into<String>) -> Self {
        Self {
            edition,
            name: name.into(),
            size_width: 0,
            size_height: 0,
            width: 0,
            height: 0,
            theater: Theater::Temperate,
            game_modes: Vec::new(),
            description_csf: String::new(),
            cells: Vec::new(),
            overlays: Vec::new(),
            terrain_objects: Vec::new(),
            entities: Vec::new(),
            waypoints: Vec::new(),
        }
    }

    /// 从场景 INI（`.map` / `.mpr`）解析尺寸、剧院，并尝试解码地形与覆盖层。
    pub fn parse_ini(edition: GameEdition, name: impl Into<String>, bytes: &[u8]) -> RaResult<Self> {
        let doc = IniDocument::parse(bytes)?;
        let size = doc.get("Map", "Size").ok_or_else(|| RaError::Parse("地图缺少 [Map] Size".into()))?;
        let (size_width, size_height) = parse_size(size)?;
        // 航点 / IsoMapPack / 覆盖层落在方形游戏格空间，边长为 Size 高 + max(宽, 高)。
        let side = game_cell_grid_side(size_width, size_height);
        let theater_raw = doc.get("Map", "Theater").unwrap_or("TEMPERATE");
        let theater = Theater::parse(theater_raw)?;
        let game_modes = parse_game_modes(doc.get("Basic", "GameModes"));
        let description_csf = doc.get("Basic", "Description").unwrap_or("").trim().to_string();
        let cells = match decode_iso_map_pack(&doc) {
            Ok(c) => c,
            Err(_) => Vec::new(),
        };
        let overlays = match decode_overlay_packs(&doc) {
            Ok(o) => o,
            Err(_) => Vec::new(),
        };
        let terrain_objects = parse_terrain_objects(&doc);
        let entities = parse_map_entities(&doc);
        let waypoints = parse_waypoints(&doc);
        Ok(Self {
            edition,
            name: name.into(),
            size_width,
            size_height,
            width: side,
            height: side,
            theater,
            game_modes,
            description_csf,
            cells,
            overlays,
            terrain_objects,
            entities,
            waypoints,
        })
    }
}

/// 解析 `[Basic] GameModes` 逗号列表（去空白、丢空段）。
pub fn parse_game_modes(raw: Option<&str>) -> Vec<String> {
    let Some(raw) = raw
    else {
        return Vec::new();
    };
    raw.split(',')
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(str::to_string)
        .collect()
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

/// 解析 `Size=x,y,width,height` 中的宽高。
fn parse_size(raw: &str) -> RaResult<(u32, u32)> {
    let parts: Vec<&str> = raw.split(',').map(str::trim).collect();
    if parts.len() < 4 {
        return Err(RaError::Parse(format!("无效 Size: {raw}")));
    }
    let width: u32 = parts[2].parse().map_err(|_| RaError::Parse(format!("Size 宽无效: {}", parts[2])))?;
    let height: u32 = parts[3].parse().map_err(|_| RaError::Parse(format!("Size 高无效: {}", parts[3])))?;
    Ok((width, height))
}
