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
mod skirmish_preview;
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
    BOOT_MAP_CANDIDATES, BootMapCandidate, BootMapResult, count_skirmish_start_slots, find_boot_map, find_boot_map_named, find_first_boot_map,
    list_parseable_boot_maps, mount_theater_mixes, skirmish_ai_row_count, try_parse_boot_map,
};
pub use compose::{TerrainImage, TileBlit, compose_terrain_rgba, paint_cell_sprites, paint_overlay_markers};
pub use fallback_preview::{RawRgbaImage, load_fallback_theater_tile, load_fallback_unit_sprite};
pub use iso_math::{HEIGHT_STEP, TILE_HEIGHT, TILE_WIDTH, iso_to_screen, screen_to_iso};
pub use iso_pack::{IsoCell, decode_iso_map_pack, parse_iso_cells};
pub use land::{LandType, ground_passable};
pub use mobile_paint::paint_map_mobiles;
pub use overlay::{NO_OVERLAY, OVERLAY_CELLS, OVERLAY_GRID, OverlayCell, decode_overlay_packs};
pub use overlay_paint::paint_map_overlays;
pub use pass_grid::{MAX_GROUND_CLIMB, PassGrid};
pub use placements::{MapEntity, MapEntityKind, parse_map_entities};
pub use skirmish_preview::{BootPreviewResult, SkirmishPreviewStats, compose_boot_preview, compose_skirmish_preview};
pub use structure_paint::paint_map_structures;
pub use terrain_objects::{TerrainObject, parse_terrain_objects};
pub use terrain_paint::paint_map_terrain_objects;
pub use terrain_preview::compose_terrain_preview;
pub use theater::{
    Theater, new_theater_shp_name, theater_ini_name, theater_mix_names, theater_new_letter, theater_palette, theater_tmp_extension,
};
pub use tileset::{TilesetLookup, parse_tileset_ini};
pub use tmp_pass::seal_pass_grid_from_tmp;
pub use waypoints::{Waypoint, parse_waypoints};

/// 地图基本信息（可附带已解码的 IsoMapPack / Overlay / Terrain / 放置 / 航点）。
#[derive(Debug, Clone)]
pub struct MapInfo {
    /// 游戏版本。
    pub edition: GameEdition,
    /// 地图名（通常为文件名）。
    pub name: String,
    /// 地图宽（格）。
    pub width: u32,
    /// 地图高（格）。
    pub height: u32,
    /// 剧院。
    pub theater: Theater,
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
            width: 0,
            height: 0,
            theater: Theater::Temperate,
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
        let (width, height) = parse_size(size)?;
        let theater_raw = doc.get("Map", "Theater").unwrap_or("TEMPERATE");
        let theater = Theater::parse(theater_raw)?;
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
        Ok(Self { edition, name: name.into(), width, height, theater, cells, overlays, terrain_objects, entities, waypoints })
    }
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
