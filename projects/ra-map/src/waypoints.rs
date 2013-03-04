//! 地图 `[Waypoints]`：任务 / 出生点等格子锚点。

use ra_assets::IniDocument;

/// 一个航点。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Waypoint {
    pub index: u32,
    pub x: u16,
    pub y: u16,
}

/// 解析 `[Waypoints]`：值为 `y * 1000 + x`（十进制）。
pub fn parse_waypoints(doc: &IniDocument) -> Vec<Waypoint> {
    let Some(section) = doc.sections.get("Waypoints") else {
        return Vec::new();
    };
    let mut out = Vec::new();
    for (key, value) in &section.order {
        let Ok(index) = key.parse::<u32>() else {
            continue;
        };
        let Ok(pos) = value.trim().parse::<u32>() else {
            continue;
        };
        let y = (pos / 1000) as u16;
        let x = (pos % 1000) as u16;
        out.push(Waypoint { index, x, y });
    }
    out.sort_by_key(|w| w.index);
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_sorted_waypoints() {
        let doc = IniDocument::parse(b"[Waypoints]\n2=4005\n0=98057\n").unwrap();
        let wp = parse_waypoints(&doc);
        assert_eq!(wp.len(), 2);
        assert_eq!(wp[0], Waypoint { index: 0, x: 57, y: 98 });
        assert_eq!(wp[1], Waypoint { index: 2, x: 5, y: 4 });
    }
}
