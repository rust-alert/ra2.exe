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

/// 遭遇战开局席位 `slot`（通常 0..=7）对应的地图航点。
///
/// 按航点编号精确匹配；缺失时返回 `None`（不得静默改用其它编号）。
pub fn skirmish_start_waypoint(waypoints: &[Waypoint], slot: u32) -> Option<Waypoint> {
    waypoints.iter().copied().find(|w| w.index == slot)
}

#[cfg(test)]
mod tests {
    use super::*;
    use ra_assets::IniDocument;

    #[test]
    fn skirmish_start_waypoint_matches_slot_index() {
        let doc = IniDocument::parse(b"[Waypoints]\n0=5002\n1=8005\n8=100\n").expect("ini");
        let wps = parse_waypoints(&doc);
        assert_eq!(skirmish_start_waypoint(&wps, 0), Some(Waypoint { index: 0, x: 2, y: 5 }));
        assert_eq!(skirmish_start_waypoint(&wps, 1), Some(Waypoint { index: 1, x: 5, y: 8 }));
        assert_eq!(skirmish_start_waypoint(&wps, 2), None);
        assert_eq!(skirmish_start_waypoint(&wps, 8), Some(Waypoint { index: 8, x: 100, y: 0 }));
    }
}
