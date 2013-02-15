//! 地图 / 剧院。

mod base64;
mod compose;
mod iso_math;
mod iso_pack;
mod lzo;
mod theater;
mod tileset;

use ra_assets::IniDocument;
use ra_types::{GameEdition, RaError, RaResult};

pub use compose::{compose_terrain_rgba, TerrainImage, TileBlit};
pub use iso_math::{iso_to_screen, HEIGHT_STEP, TILE_HEIGHT, TILE_WIDTH};
pub use iso_pack::{decode_iso_map_pack, IsoCell};
pub use theater::{
    theater_ini_name, theater_mix_names, theater_palette, theater_tmp_extension, Theater,
};
pub use tileset::{parse_tileset_ini, TilesetLookup};

/// 地图基本信息（可附带已解码的 IsoMapPack 单元）。
#[derive(Debug, Clone)]
pub struct MapInfo {
    pub edition: GameEdition,
    pub name: String,
    pub width: u32,
    pub height: u32,
    pub theater: Theater,
    pub cells: Vec<IsoCell>,
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
        }
    }

    /// 从场景 INI（`.map` / `.mpr`）解析尺寸、剧院，并尝试解码 IsoMapPack5。
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
        Ok(Self {
            edition,
            name: name.into(),
            width,
            height,
            theater,
            cells,
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
