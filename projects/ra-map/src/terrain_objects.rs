//! 地图 `[Terrain]`：树 / 岩石等静态物件占位。

use ra_assets::IniDocument;

/// 一处地形物件。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TerrainObject {
    /// 格子 X。
    pub x: u16,
    /// 格子 Y。
    pub y: u16,
    /// 物件类型名（通常已大写）。
    pub name: String,
}

/// 解析 `[Terrain]`：键为 `y * 1000 + x`，值为物件类型名。
pub fn parse_terrain_objects(doc: &IniDocument) -> Vec<TerrainObject> {
    let Some(section) = doc.section("Terrain")
    else {
        return Vec::new();
    };
    let mut out = Vec::new();
    for (key, value) in section.pairs() {
        let Ok(pos) = key.parse::<u32>()
        else {
            continue;
        };
        let name = value.trim();
        if name.is_empty() {
            continue;
        }
        let y = (pos / 1000) as u16;
        let x = (pos % 1000) as u16;
        out.push(TerrainObject { x, y, name: name.to_ascii_uppercase() });
    }
    out
}
