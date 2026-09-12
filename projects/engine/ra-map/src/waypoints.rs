//! 地图 `[Waypoints]`：任务 / 出生点等格子锚点。

use ra_assets::IniDocument;

use crate::packed_cell::parse_packed_cell;

/// 一个航点。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[doc(hidden)]
pub struct Waypoint {
    /// 航点编号。
    pub index: u32,
    /// 格子 X。
    pub x: u16,
    /// 格子 Y。
    pub y: u16,
}

/// 解析 `[Waypoints]`：值为 `y * 1000 + x`（十进制）。
pub fn parse_waypoints(doc: &IniDocument) -> Vec<Waypoint> {
    let Some(section) = doc.section("Waypoints")
    else {
        return Vec::new();
    };
    let mut out = Vec::new();
    for (key, value) in section.pairs() {
        let Ok(index) = key.trim().parse::<u32>()
        else {
            continue;
        };
        let Some((x, y)) = parse_packed_cell(value)
        else {
            continue;
        };
        out.push(Waypoint { index, x, y });
    }
    out.sort_by_key(|w| w.index);
    out
}

/// 遭遇战开局席位 `slot`（通常 0..=7）对应的地图航点。
///
/// 按航点编号精确匹配；缺失时返回 `None`（不得静默改用其它编号）。
pub fn skirmish_start_waypoint(waypoints: &[Waypoint], slot: u32) -> Option<Waypoint> {
    waypoints.iter().copied().find(|w| w.index == slot)
}
