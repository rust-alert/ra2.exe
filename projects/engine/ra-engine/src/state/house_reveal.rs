//! 按阵营记录的地图揭示格（超武 / 触发竖切；无全图迷雾渲染依赖）。

use std::collections::{BTreeMap, BTreeSet};

/// 各 house 已揭示单元格集合。
#[derive(Debug, Clone, Default)]
pub struct HouseRevealState {
    by_house: BTreeMap<String, BTreeSet<(u16, u16)>>,
}

impl HouseRevealState {
    /// 以 `(cx, cy)` 为中心、切比雪夫 `radius` 画圆盘写入揭示集（裁到地图边界）。
    pub fn reveal_disk(&mut self, house: &str, cx: u16, cy: u16, radius: u32, map_w: u32, map_h: u32) {
        let house_key = house.trim().to_ascii_uppercase();
        if house_key.is_empty() || map_w == 0 || map_h == 0 {
            return;
        }
        let r = radius as i32;
        let set = self.by_house.entry(house_key).or_default();
        for dy in -r..=r {
            for dx in -r..=r {
                if dx.abs().max(dy.abs()) > r {
                    continue;
                }
                let x = cx as i32 + dx;
                let y = cy as i32 + dy;
                if x < 0 || y < 0 || x >= map_w as i32 || y >= map_h as i32 {
                    continue;
                }
                set.insert((x as u16, y as u16));
            }
        }
    }

    /// 该格是否已被该 house 揭示。
    pub fn is_revealed(&self, house: &str, x: u16, y: u16) -> bool {
        let house_key = house.trim().to_ascii_uppercase();
        self.by_house.get(&house_key).is_some_and(|s| s.contains(&(x, y)))
    }

    /// 该 house 已揭示格数。
    pub fn revealed_count(&self, house: &str) -> usize {
        let house_key = house.trim().to_ascii_uppercase();
        self.by_house.get(&house_key).map(|s| s.len()).unwrap_or(0)
    }
}
