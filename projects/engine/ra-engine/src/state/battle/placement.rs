use ra_map::{LandType, MapEntityKind};
use ra_types::{TypeId, occupancy_kind};

use super::types::BattleState;

impl BattleState {
    /// 目标格是否可放置单格**陆地**建筑（界内、可通行、无占用实体）。
    ///
    /// 单位 FreeUnit / 工厂出兵等仍走此入口；水域建筑请用 [`Self::can_place_structure_footprint`]。
    pub fn can_place_structure(&self, x: u16, y: u16) -> bool {
        self.can_place_structure_footprint(x, y, 1, 1, false)
    }

    /// 以 `(x,y)` 为左上角，检查 `width×height` 矩形是否全部可放置。
    ///
    /// - `water_bound=false`：每格须陆地可通行（`is_passable`）。
    /// - `water_bound=true`：每格须规范陆地为 [`LandType::Water`]（不要求地面通行）。
    ///
    /// 不含建区邻接；玩家 / AI 落建筑请用 [`Self::can_place_building_for`]。
    pub fn can_place_structure_footprint(&self, x: u16, y: u16, width: u16, height: u16, water_bound: bool) -> bool {
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
                if !self.cell_ok_for_structure(cx, cy, water_bound) {
                    return false;
                }
            }
        }
        true
    }

    /// 占地几何 + 己方建区（`BaseNormal`/`Adjacent`）或墙链（`Wall`/`GuardRange`）。
    ///
    /// 人类玩家落位入口；AI 请用 [`Self::can_place_building_for_ai`]。
    pub fn can_place_building_for(&self, house: &str, type_id: TypeId, x: u16, y: u16) -> bool {
        let Some(sdef) = self.definitions.structures.get_by_id(type_id)
        else {
            return false;
        };
        let foundation = &sdef.foundation;
        if !self.can_place_structure_footprint(x, y, foundation.width, foundation.height, sdef.water_bound) {
            return false;
        }
        if self.house_build_zone_allows(house, sdef.adjacent, x, y, foundation.width, foundation.height) {
            return true;
        }
        // 围墙：可沿己方同型墙在 `GuardRange` 内正交延伸（含自动补段路径畅通）。
        sdef.wall && self.wall_chain_anchor(house, type_id, sdef.guard_range, x, y).is_some()
    }

    /// AI 落位：占地几何 + `AIBaseSpacing` 最少空隙（可选优先多一格）+ 船厂最大距。
    ///
    /// 不走人类 `Adjacent` 建区；墙仍可用 `GuardRange` 链。`min_gap_cells` 为足迹间最少空隙格数
    ///（切比雪夫 `d >= min_gap_cells + 1`）。围墙不参与间距锚点，避免墙把基地撑开。
    pub fn can_place_building_for_ai(&self, house: &str, type_id: TypeId, x: u16, y: u16, min_gap_cells: u32) -> bool {
        let Some(sdef) = self.definitions.structures.get_by_id(type_id)
        else {
            return false;
        };
        let foundation = &sdef.foundation;
        let width = foundation.width.max(1);
        let height = foundation.height.max(1);
        if !self.can_place_structure_footprint(x, y, width, height, sdef.water_bound) {
            return false;
        }
        if sdef.water_bound && !self.house_naval_yard_adjacency_allows(house, x, y, width, height) {
            return false;
        }
        if sdef.wall && self.wall_chain_anchor(house, type_id, sdef.guard_range, x, y).is_some() {
            return true;
        }
        self.house_ai_spacing_allows(house, min_gap_cells, x, y, width, height)
    }

    /// 相对己方非墙建筑是否满足最少空隙（`min_gap_cells`；`0` 表示可贴边）。
    pub fn house_ai_spacing_allows(&self, house: &str, min_gap_cells: u32, x: u16, y: u16, width: u16, height: u16) -> bool {
        let Some(house_id) = crate::gameplay::house_id_of(&self.definitions, house)
        else {
            return false;
        };
        let width = width.max(1);
        let height = height.max(1);
        let min_d = min_gap_cells.saturating_add(1);
        use crate::state::components::{Health, Identity, Owner, Transform};

        // 尚无其它非墙建筑时，只要几何可放即可（开局贴建造场）。
        let mut saw_anchor = false;
        for e in &self.entities {
            let id = e.id;
            if self.ecs_get::<Health>(id).map(|h| h.dead).unwrap_or(true) {
                continue;
            }
            if !self.ecs_get::<Owner>(id).is_some_and(|o| o.house == house_id) {
                continue;
            }
            let Some(identity) = self.ecs_get::<Identity>(id)
            else {
                continue;
            };
            if identity.kind != MapEntityKind::Structure {
                continue;
            }
            let Some(anchor) = self.definitions.structures.get_by_id(identity.type_id)
            else {
                continue;
            };
            if anchor.wall {
                continue;
            }
            let Some(xf) = self.ecs_get::<Transform>(id)
            else {
                continue;
            };
            saw_anchor = true;
            let aw = anchor.foundation.width.max(1);
            let ah = anchor.foundation.height.max(1);
            let d = min_chebyshev_between_footprints(x, y, width, height, xf.x, xf.y, aw, ah);
            if d < min_d {
                return false;
            }
        }
        saw_anchor || min_gap_cells == 0
    }

    /// AI 船厂是否落在建造场 `AINavalYardAdjacency` 切比雪夫半径内。
    pub fn house_naval_yard_adjacency_allows(&self, house: &str, x: u16, y: u16, width: u16, height: u16) -> bool {
        let Some(house_id) = crate::gameplay::house_id_of(&self.definitions, house)
        else {
            return false;
        };
        let width = width.max(1);
        let height = height.max(1);
        let max_d = self.definitions.ai_naval_yard_adjacency.max(1);
        use crate::state::components::{Health, Identity, Owner, Transform};

        self.entities.iter().any(|e| {
            let id = e.id;
            if self.ecs_get::<Health>(id).map(|h| h.dead).unwrap_or(true) {
                return false;
            }
            if !self.ecs_get::<Owner>(id).is_some_and(|o| o.house == house_id) {
                return false;
            }
            let Some(identity) = self.ecs_get::<Identity>(id)
            else {
                return false;
            };
            if identity.kind != MapEntityKind::Structure {
                return false;
            }
            let Some(anchor) = self.definitions.structures.get_by_id(identity.type_id)
            else {
                return false;
            };
            if !anchor.construction_yard {
                return false;
            }
            let Some(xf) = self.ecs_get::<Transform>(id)
            else {
                return false;
            };
            let aw = anchor.foundation.width.max(1);
            let ah = anchor.foundation.height.max(1);
            min_chebyshev_between_footprints(x, y, width, height, xf.x, xf.y, aw, ah) <= max_d
        })
    }

    /// 墙落位时实际生成的格列表（含点击格；中间格为免费补段）。
    ///
    /// 非墙或无可用锚点时仅返回点击格（调用方仍须先通过放置校验）。
    pub fn wall_placement_cells(&self, house: &str, type_id: TypeId, x: u16, y: u16) -> Vec<(u16, u16)> {
        let Some(sdef) = self.definitions.structures.get_by_id(type_id)
        else {
            return vec![(x, y)];
        };
        if !sdef.wall {
            return vec![(x, y)];
        }
        match self.wall_chain_anchor(house, type_id, sdef.guard_range, x, y) {
            Some((ax, ay)) => orthogonal_cells_between(ax, ay, x, y).into_iter().chain(std::iter::once((x, y))).collect(),
            None => vec![(x, y)],
        }
    }

    /// 新占地是否落在己方 `BaseNormal=yes` 建筑的 `Adjacent` 建区内。
    ///
    /// 距离按足迹间最小切比雪夫距离 `d`：`d <= Adjacent + 1`（`Adjacent=0` 须贴边，`=1` 最多隔一格）。
    /// `adjacent < 0` 时永久拒绝。
    pub fn house_build_zone_allows(&self, house: &str, adjacent: i32, x: u16, y: u16, width: u16, height: u16) -> bool {
        if adjacent < 0 {
            return false;
        }
        let Some(house_id) = crate::gameplay::house_id_of(&self.definitions, house)
        else {
            return false;
        };
        let width = width.max(1);
        let height = height.max(1);
        let max_d = (adjacent as u32).saturating_add(1);
        use crate::state::components::{Health, Identity, Owner, Transform};

        self.entities.iter().any(|e| {
            let id = e.id;
            if self.ecs_get::<Health>(id).map(|h| h.dead).unwrap_or(true) {
                return false;
            }
            if !self.ecs_get::<Owner>(id).is_some_and(|o| o.house == house_id) {
                return false;
            }
            let Some(identity) = self.ecs_get::<Identity>(id)
            else {
                return false;
            };
            if identity.kind != MapEntityKind::Structure {
                return false;
            }
            let Some(anchor) = self.definitions.structures.get_by_id(identity.type_id)
            else {
                return false;
            };
            if !anchor.base_normal {
                return false;
            }
            let Some(xf) = self.ecs_get::<Transform>(id)
            else {
                return false;
            };
            let aw = anchor.foundation.width.max(1);
            let ah = anchor.foundation.height.max(1);
            min_chebyshev_between_footprints(x, y, width, height, xf.x, xf.y, aw, ah) <= max_d
        })
    }

    /// 寻找可与 `(x,y)` 正交相连的己方同型墙锚点（轴距 ≤ 有效 `GuardRange`，中间格可放）。
    ///
    /// 多候选时取轴距最短者。仅支持 `1x1` 墙枢纽。
    fn wall_chain_anchor(&self, house: &str, type_id: TypeId, guard_range: i32, x: u16, y: u16) -> Option<(u16, u16)> {
        let max_dist = effective_guard_range_cells(guard_range);
        if max_dist == 0 {
            return None;
        }
        let Some(house_id) = crate::gameplay::house_id_of(&self.definitions, house)
        else {
            return None;
        };
        use crate::state::components::{Health, Identity, Owner, Transform};

        let mut best: Option<(u32, u16, u16)> = None;
        for e in &self.entities {
            let id = e.id;
            if self.ecs_get::<Health>(id).map(|h| h.dead).unwrap_or(true) {
                continue;
            }
            if !self.ecs_get::<Owner>(id).is_some_and(|o| o.house == house_id) {
                continue;
            }
            let Some(identity) = self.ecs_get::<Identity>(id)
            else {
                continue;
            };
            if identity.kind != MapEntityKind::Structure || identity.type_id != type_id {
                continue;
            }
            let Some(anchor) = self.definitions.structures.get_by_id(identity.type_id)
            else {
                continue;
            };
            if !anchor.wall {
                continue;
            }
            // 仅 1x1 枢纽参与正交补段（闸门等多格墙不走自动填缝）。
            if anchor.foundation.width.max(1) != 1 || anchor.foundation.height.max(1) != 1 {
                continue;
            }
            let Some(xf) = self.ecs_get::<Transform>(id)
            else {
                continue;
            };
            let Some(dist) = orthogonal_axis_distance(xf.x, xf.y, x, y)
            else {
                continue;
            };
            if dist == 0 || dist > max_dist {
                continue;
            }
            if !self.wall_fill_path_clear(xf.x, xf.y, x, y, anchor.water_bound) {
                continue;
            }
            if best.map(|(d, _, _)| dist < d).unwrap_or(true) {
                best = Some((dist, xf.x, xf.y));
            }
        }
        best.map(|(_, ax, ay)| (ax, ay))
    }

    /// 锚点与落点之间的中间格是否全部可放（不含两端）。
    fn wall_fill_path_clear(&self, ax: u16, ay: u16, bx: u16, by: u16, water_bound: bool) -> bool {
        orthogonal_cells_between(ax, ay, bx, by).into_iter().all(|(cx, cy)| self.cell_ok_for_structure(cx, cy, water_bound))
    }

    /// 单格是否满足建筑落位的陆地 / 水域条件（含实体占用）。
    pub fn cell_ok_for_structure(&self, cx: u16, cy: u16, water_bound: bool) -> bool {
        if !self.pass_grid.in_bounds(cx, cy) {
            return false;
        }
        if self.cell_blocked_by_entity(cx, cy) {
            return false;
        }
        if water_bound { self.pass_grid.land_type(cx, cy) == LandType::Water } else { self.pass_grid.is_passable(cx, cy) }
    }

    /// 格上是否有存活实体占用：机动单位看锚点格，建筑看完整 `Foundation` 矩形。
    fn cell_blocked_by_entity(&self, cx: u16, cy: u16) -> bool {
        use crate::state::components::{Health, Identity, Transform};

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
            let type_id = self.ecs_get::<Identity>(id).map(|i| i.type_id);
            let foundation =
                type_id.and_then(|tid| self.definitions.structures.get_by_id(tid)).map(|s| s.foundation.clone()).unwrap_or_default();
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
    ///
    /// 通行按规范陆地恢复：水域 / 岩 / 墙等仍不可走，避免船厂拆除后水面被误开。
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
                    let passable = ra_map::land_passable(self.pass_grid.land_type(cx, cy));
                    self.pass_grid.set_passable(cx, cy, passable);
                    self.set_prepared_passable(cx, cy, passable);
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

/// 原版 `GuardRange` 有效格数：负值按 0；超过 [`ra_types::MAX_GUARD_RANGE_CELLS`] 截断。
pub fn effective_guard_range_cells(guard_range: i32) -> u32 {
    if guard_range <= 0 {
        return 0;
    }
    (guard_range.min(ra_types::MAX_GUARD_RANGE_CELLS)) as u32
}

/// 两点正交轴距（同行或同列）；斜向返回 `None`。
pub fn orthogonal_axis_distance(ax: u16, ay: u16, bx: u16, by: u16) -> Option<u32> {
    if ax == bx {
        Some((i32::from(ay) - i32::from(by)).unsigned_abs())
    }
    else if ay == by {
        Some((i32::from(ax) - i32::from(bx)).unsigned_abs())
    }
    else {
        None
    }
}

/// 正交线段上不含端点的中间格（由 `a` 走向 `b`）。
pub fn orthogonal_cells_between(ax: u16, ay: u16, bx: u16, by: u16) -> Vec<(u16, u16)> {
    let mut out = Vec::new();
    if ax == bx {
        let (lo, hi) = if ay < by { (ay, by) } else { (by, ay) };
        let mut y = lo.saturating_add(1);
        while y < hi {
            out.push((ax, y));
            y = y.saturating_add(1);
        }
    }
    else if ay == by {
        let (lo, hi) = if ax < bx { (ax, bx) } else { (bx, ax) };
        let mut x = lo.saturating_add(1);
        while x < hi {
            out.push((x, ay));
            x = x.saturating_add(1);
        }
    }
    out
}

/// 两矩形足迹（左上锚点 + 宽高，含端点）之间的最小切比雪夫距离。
///
/// 重叠为 `0`；边或角相贴为 `1`；中间隔一格为 `2`。
pub fn min_chebyshev_between_footprints(ax: u16, ay: u16, aw: u16, ah: u16, bx: u16, by: u16, bw: u16, bh: u16) -> u32 {
    let aw = aw.max(1);
    let ah = ah.max(1);
    let bw = bw.max(1);
    let bh = bh.max(1);
    let a_x2 = u32::from(ax).saturating_add(u32::from(aw) - 1);
    let a_y2 = u32::from(ay).saturating_add(u32::from(ah) - 1);
    let b_x2 = u32::from(bx).saturating_add(u32::from(bw) - 1);
    let b_y2 = u32::from(by).saturating_add(u32::from(bh) - 1);
    let ax0 = u32::from(ax);
    let ay0 = u32::from(ay);
    let bx0 = u32::from(bx);
    let by0 = u32::from(by);
    let dx = if a_x2 < bx0 {
        bx0 - a_x2
    }
    else if b_x2 < ax0 {
        ax0 - b_x2
    }
    else {
        0
    };
    let dy = if a_y2 < by0 {
        by0 - a_y2
    }
    else if b_y2 < ay0 {
        ay0 - b_y2
    }
    else {
        0
    };
    dx.max(dy)
}

#[cfg(test)]
mod tests {
    use super::{effective_guard_range_cells, min_chebyshev_between_footprints, orthogonal_axis_distance, orthogonal_cells_between};

    #[test]
    fn chebyshev_touching_edge_is_one() {
        assert_eq!(min_chebyshev_between_footprints(4, 4, 1, 1, 5, 4, 1, 1), 1);
        assert_eq!(min_chebyshev_between_footprints(4, 4, 2, 2, 6, 4, 2, 2), 1);
    }

    #[test]
    fn chebyshev_one_cell_gap_is_two() {
        assert_eq!(min_chebyshev_between_footprints(4, 4, 1, 1, 6, 4, 1, 1), 2);
    }

    #[test]
    fn chebyshev_diagonal_touch_is_one() {
        assert_eq!(min_chebyshev_between_footprints(4, 4, 1, 1, 5, 5, 1, 1), 1);
    }

    #[test]
    fn chebyshev_overlap_is_zero() {
        assert_eq!(min_chebyshev_between_footprints(4, 4, 2, 2, 5, 5, 2, 2), 0);
    }

    #[test]
    fn orthogonal_distance_rejects_diagonal() {
        assert_eq!(orthogonal_axis_distance(4, 4, 4, 8), Some(4));
        assert_eq!(orthogonal_axis_distance(4, 4, 7, 4), Some(3));
        assert_eq!(orthogonal_axis_distance(4, 4, 5, 5), None);
    }

    #[test]
    fn orthogonal_cells_between_excludes_endpoints() {
        assert_eq!(orthogonal_cells_between(4, 4, 4, 7), vec![(4, 5), (4, 6)]);
        assert_eq!(orthogonal_cells_between(7, 4, 4, 4), vec![(5, 4), (6, 4)]);
        assert!(orthogonal_cells_between(4, 4, 4, 5).is_empty());
        assert!(orthogonal_cells_between(4, 4, 5, 5).is_empty());
    }

    #[test]
    fn guard_range_clamped_to_vanilla_cap() {
        assert_eq!(effective_guard_range_cells(-1), 0);
        assert_eq!(effective_guard_range_cells(0), 0);
        assert_eq!(effective_guard_range_cells(4), 4);
        assert_eq!(effective_guard_range_cells(100), 16);
    }
}
