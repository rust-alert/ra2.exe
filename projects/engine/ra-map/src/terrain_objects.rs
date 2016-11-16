//! 地图 `[Terrain]`：树 / 岩石等静态物件占位。

use ra_assets::IniDocument;
use ra_types::TerrainName;

use crate::packed_cell::parse_packed_cell;

/// 一处地形物件。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TerrainObject {
    /// 格子 X。
    pub x: u16,
    /// 格子 Y。
    pub y: u16,
    /// 物件类型名（装载期一次解码为大写地形键）。
    pub name: TerrainName,
}

/// 解析 `[Terrain]`：键为 `y * 1000 + x`，值为物件类型名。
pub fn parse_terrain_objects(doc: &IniDocument) -> Vec<TerrainObject> {
    let Some(section) = doc.section("Terrain")
    else {
        return Vec::new();
    };
    let mut out = Vec::new();
    for (key, value) in section.pairs() {
        let Some((x, y)) = parse_packed_cell(key)
        else {
            continue;
        };
        let name = TerrainName::parse(value);
        if name.is_empty() {
            continue;
        }
        out.push(TerrainObject { x, y, name });
    }
    out
}
