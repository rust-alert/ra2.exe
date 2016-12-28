use ra_types::occupancy_kind;

use super::types::BattleState;

impl BattleState {
    /// 目标格是否可放置单格建筑（界内、可通行、无占用实体）。
    pub fn can_place_structure(&self, x: u16, y: u16) -> bool {
        self.can_place_structure_footprint(x, y, 1, 1)
    }

    /// 以 `(x,y)` 为左上角，检查 `width×height` 矩形是否全部可放置。
    pub fn can_place_structure_footprint(&self, x: u16, y: u16, width: u16, height: u16) -> bool {
        let width = width.max(1);
        let height = height.max(1);
        for dy in 0..height {
            for dx in 0..width {
                let Some(cx) = x.checked_add(dx)
                else {
                    return false;
                };
                let Some(cy) = y.checked_add(dy)
                else {
                    return false;
                };
                if !self.pass_grid.in_bounds(cx, cy) {
                    return false;
                }
                if !self.pass_grid.is_passable(cx, cy) {
                    return false;
                }
                if self.cell_blocked_by_entity(cx, cy) {
                    return false;
                }
            }
        }
        true
    }

    /// 格上是否有存活实体占用：机动单位看锚点格，建筑看完整 `Foundation` 矩形。
    fn cell_blocked_by_entity(&self, cx: u16, cy: u16) -> bool {
        use crate::state::components::{Health, Identity, Transform};
        use ra_map::MapEntityKind;

        self.entities.iter().any(|e| {
            let id = e.id;
            if self.ecs_get::<Health>(id).map(|h| h.dead).unwrap_or(true) {
                return false;
            }
            let Some(xf) = self.ecs_get::<Transform>(id)
            else {
                return false;
            };
            let is_structure = self.ecs_get::<Identity>(id).map(|i| i.kind == MapEntityKind::Structure).unwrap_or(false);
            if !is_structure {
                return xf.x == cx && xf.y == cy;
            }
            let type_id = self.ecs_get::<Identity>(id).map(|i| i.type_id.clone()).unwrap_or_default();
            let foundation = self.definitions.structures.get(type_id.as_ref()).map(|s| s.foundation.clone()).unwrap_or_default();
            let fw = foundation.width.max(1);
            let fh = foundation.height.max(1);
            cx >= xf.x && cy >= xf.y && cx < xf.x.saturating_add(fw) && cy < xf.y.saturating_add(fh)
        })
    }

    /// 将建筑占地矩形全部标为不可通行，并同步 `prepared.passable` / `prepared.occupancy`。
    pub fn seal_structure_footprint(&mut self, x: u16, y: u16, width: u16, height: u16) {
        let width = width.max(1);
        let height = height.max(1);
        for dy in 0..height {
            for dx in 0..width {
                let Some(cx) = x.checked_add(dx)
                else {
                    continue;
                };
                let Some(cy) = y.checked_add(dy)
                else {
                    continue;
                };
                if self.pass_grid.in_bounds(cx, cy) {
                    self.pass_grid.set_passable(cx, cy, false);
                    self.set_prepared_passable(cx, cy, false);
                    self.set_prepared_occupancy(cx, cy, occupancy_kind::STRUCTURE);
                }
            }
        }
    }

    /// 出售 / 拆除后释放建筑占地通行，并恢复静态占格（地形 / 污迹 / 空）。
    pub fn unseal_structure_footprint(&mut self, x: u16, y: u16, width: u16, height: u16) {
        let width = width.max(1);
        let height = height.max(1);
        for dy in 0..height {
            for dx in 0..width {
                let Some(cx) = x.checked_add(dx)
                else {
                    continue;
                };
                let Some(cy) = y.checked_add(dy)
                else {
                    continue;
                };
                if self.pass_grid.in_bounds(cx, cy) {
                    self.pass_grid.set_passable(cx, cy, true);
                    self.set_prepared_passable(cx, cy, true);
                    if self.prepared_occupancy_at(cx, cy) == Some(occupancy_kind::STRUCTURE) {
                        self.set_prepared_occupancy(cx, cy, self.static_occupancy_at(cx, cy));
                    }
                }
            }
        }
    }

    fn prepared_cell_index(&self, x: u16, y: u16) -> Option<usize> {
        let w = self.prepared.pass_width;
        let h = self.prepared.pass_height;
        if w == 0 || h == 0 {
            return None;
        }
        if u32::from(x) >= w || u32::from(y) >= h {
            return None;
        }
        Some((u32::from(y) * w + u32::from(x)) as usize)
    }

    fn prepared_occupancy_at(&self, x: u16, y: u16) -> Option<u8> {
        let i = self.prepared_cell_index(x, y)?;
        self.prepared.occupancy.get(i).copied()
    }

    fn set_prepared_occupancy(&mut self, x: u16, y: u16, kind: u8) {
        let Some(i) = self.prepared_cell_index(x, y)
        else {
            return;
        };
        if let Some(cell) = self.prepared.occupancy.get_mut(i) {
            *cell = kind;
        }
    }

    fn set_prepared_passable(&mut self, x: u16, y: u16, passable: bool) {
        let Some(i) = self.prepared_cell_index(x, y)
        else {
            return;
        };
        if let Some(cell) = self.prepared.passable.get_mut(i) {
            *cell = if passable { 1 } else { 0 };
        }
    }

    /// 地图静态层占格：地形物件优先于污迹（与装载 `prepared_occupancy_from_map` 一致）。
    fn static_occupancy_at(&self, x: u16, y: u16) -> u8 {
        if self.map.terrain_objects.iter().any(|t| t.x == x && t.y == y) {
            return occupancy_kind::TERRAIN;
        }
        if self.map.smudges.iter().any(|s| s.x == x && s.y == y) {
            return occupancy_kind::SMUDGE;
        }
        occupancy_kind::EMPTY
    }
}
