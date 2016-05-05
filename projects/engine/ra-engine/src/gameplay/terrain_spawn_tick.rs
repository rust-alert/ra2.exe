//! `BattleState` 上的矿柱产矿 tick。

use super::terrain_spawn::{TerrainSpawnerTick, place_spawned_ore, terrain_spawn_sample};
use crate::state::BattleState;

impl BattleState {
    /// 推进全部矿柱产矿动画；中点时在邻格写入可采 overlay。
    pub(crate) fn advance_terrain_spawners(&mut self) {
        if self.terrain_spawners.is_empty() {
            return;
        }
        let seed = self.match_seed;
        let tick = self.tick;
        let mut spawn_cells = Vec::new();
        for spawner in &mut self.terrain_spawners {
            let sample = terrain_spawn_sample(seed, tick, spawner.x, spawner.y);
            if spawner.tick(sample) == TerrainSpawnerTick::SpawnDue {
                spawn_cells.push((spawner.x, spawner.y));
            }
        }
        for (x, y) in spawn_cells {
            let in_bounds = |cx: u16, cy: u16| self.pass_grid.in_bounds(cx, cy);
            if let Some(cell) = place_spawned_ore(&mut self.map.overlays, &self.overlay_types, x, y, in_bounds) {
                self.overlay_paint_dirty.push(cell);
            }
        }
    }

    /// 取出产矿等写入后待叠画的 overlay 格子（呈现层消费）。
    pub fn take_overlay_paint_dirty(&mut self) -> Vec<(u16, u16)> {
        std::mem::take(&mut self.overlay_paint_dirty)
    }

    /// 用 SHP 实测总帧数回写矿柱（boot 烘焙银行后调用）。
    pub fn apply_ore_tree_frame_counts(&mut self, counts: &[(u16, u16, u16)]) {
        for &(x, y, frames) in counts {
            if let Some(s) = self.terrain_spawners.iter_mut().find(|s| s.x == x && s.y == y) {
                if frames > 0 {
                    s.frame_count = frames;
                    s.midpoint_frame = frames / 2;
                }
            }
        }
    }
}
