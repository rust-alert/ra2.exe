//! 地图 / 剧院。

mod base64;
mod boot_map;
mod compose;
mod iso_math;
mod iso_pack;
mod land;
mod lcw;
mod lzo;
mod overlay;
mod pass_grid;
mod placements;
mod theater;
mod terrain_objects;
mod tileset;
mod tmp_pass;
mod waypoints;

use ra_assets::IniDocument;
use ra_types::{GameEdition, RaError, RaResult};

pub use boot_map::{mount_theater_mixes, try_parse_boot_map, BOOT_MAP_CANDIDATES};
pub use compose::{
    compose_terrain_rgba, paint_cell_sprites, paint_overlay_markers, TerrainImage, TileBlit,
};
pub use iso_math::{iso_to_screen, screen_to_iso, HEIGHT_STEP, TILE_HEIGHT, TILE_WIDTH};
pub use iso_pack::{decode_iso_map_pack, IsoCell};
pub use land::{ground_passable, LandType};
pub use overlay::{decode_overlay_packs, OverlayCell, NO_OVERLAY, OVERLAY_CELLS, OVERLAY_GRID};
pub use pass_grid::{PassGrid, MAX_GROUND_CLIMB};
pub use placements::{parse_map_entities, MapEntity, MapEntityKind};
pub use theater::{
    new_theater_shp_name, theater_ini_name, theater_mix_names, theater_new_letter,
    theater_palette, theater_tmp_extension, Theater,
};
pub use terrain_objects::{parse_terrain_objects, TerrainObject};
pub use tileset::{parse_tileset_ini, TilesetLookup};
pub use tmp_pass::seal_pass_grid_from_tmp;
pub use waypoints::{parse_waypoints, Waypoint};

/// 地图基本信息（可附带已解码的 IsoMapPack / Overlay / Terrain / 放置 / 航点）。
#[derive(Debug, Clone)]
pub struct MapInfo {
    pub edition: GameEdition,
    pub name: String,
    pub width: u32,
    pub height: u32,
    pub theater: Theater,
    pub cells: Vec<IsoCell>,
    pub overlays: Vec<OverlayCell>,
    pub terrain_objects: Vec<TerrainObject>,
    pub entities: Vec<MapEntity>,
    pub waypoints: Vec<Waypoint>,
}

impl MapInfo {
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
        let size = doc
            .get("Map", "Size")
            .ok_or_else(|| RaError::Parse("地图缺少 [Map] Size".into()))?;
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
        Ok(Self {
            edition,
            name: name.into(),
            width,
            height,
            theater,
            cells,
            overlays,
            terrain_objects,
            entities,
            waypoints,
        })
    }
}

/// `Size=x,y,width,height`
fn parse_size(raw: &str) -> RaResult<(u32, u32)> {
    let parts: Vec<&str> = raw.split(',').map(str::trim).collect();
    if parts.len() < 4 {
        return Err(RaError::Parse(format!("无效 Size: {raw}")));
    }
    let width: u32 = parts[2]
        .parse()
        .map_err(|_| RaError::Parse(format!("Size 宽无效: {}", parts[2])))?;
    let height: u32 = parts[3]
        .parse()
        .map_err(|_| RaError::Parse(format!("Size 高无效: {}", parts[3])))?;
    Ok((width, height))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_basic_map_ini() {
        let text = b"[Map]\nSize=0,0,50,40\nTheater=SNOW\n";
        let info = MapInfo::parse_ini(GameEdition::Ra2, "t", text).unwrap();
        assert_eq!(info.width, 50);
        assert_eq!(info.height, 40);
        assert_eq!(info.theater, Theater::Snow);
        assert!(info.cells.is_empty());
    }
}
