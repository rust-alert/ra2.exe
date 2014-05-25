//! 地图 `[Waypoints]`：任务 / 出生点等格子锚点。

use ra_assets::IniDocument;

/// 一个航点。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
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
        let Ok(index) = key.parse::<u32>()
        else {
            continue;
        };
        let Ok(pos) = value.trim().parse::<u32>()
        else {
            continue;
        };
        let y = (pos / 1000) as u16;
        let x = (pos % 1000) as u16;
        out.push(Waypoint { index, x, y });
    }
    out.sort_by_key(|w| w.index);
    out
}
