//! 地图 `[Smudge]`：弹坑 / 焦痕等污迹占位。

use ra_assets::IniDocument;

use crate::packed_cell::parse_packed_cell;

/// 一处污迹（装载解析中间态；投影进 `ra_types::MapSmudge`）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MapSmudge {
    /// 格子 X。
    pub x: u16,
    /// 格子 Y。
    pub y: u16,
    /// 污迹类型名（通常已大写）。
    pub name: String,
}

/// 解析 `[Smudge]`：键为 `y * 1000 + x`，值为污迹类型名。
pub fn parse_map_smudges(doc: &IniDocument) -> Vec<MapSmudge> {
    let Some(section) = doc.section("Smudge")
    else {
        return Vec::new();
    };
    let mut out = Vec::new();
    for (key, value) in section.pairs() {
        let Some((x, y)) = parse_packed_cell(key)
        else {
            continue;
        };
        let name = value.trim();
        if name.is_empty() {
            continue;
        }
        out.push(MapSmudge { x, y, name: name.to_ascii_uppercase() });
    }
    out
}
