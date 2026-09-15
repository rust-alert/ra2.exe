//! 单位移动、朝向与路径辅助函数。

use ra_map::{MapEntityKind, PassGrid};

use crate::state::components::{AttackState, CombatStats, Health, Identity, Locomotor, MovementState, Transform};

#[doc(hidden)]
pub fn is_mobile(kind: MapEntityKind) -> bool {
    matches!(kind, MapEntityKind::Unit | MapEntityKind::Infantry | MapEntityKind::Aircraft)
}

impl crate::state::BattleState {
    /// 实体是否 `Naval=yes`（海军单位只走水面）。
    pub fn entity_is_naval(&self, id: ra_types::EntityId) -> bool {
        self.ecs_get::<Identity>(id).and_then(|identity| self.definitions.techno.get_by_id(identity.type_id)).map(|t| t.naval).unwrap_or(false)
    }

    /// 为机动单位准备寻路用通行表：海军先按水面改写，再封建筑占地与其它单位。
    fn prepare_move_grid(&self, mover_index: usize, naval: bool) -> PassGrid {
        let mut grid = self.pass_grid.clone();
        if naval {
            grid.remap_passable_for_naval();
            self.reseal_structure_footprints_on_grid(&mut grid);
        }
        for (j, entity) in self.entities.iter().enumerate() {
            if j == mover_index {
                continue;
            }
            let oid = entity.id;
            if self.ecs_get::<Health>(oid).map(|h| h.dead).unwrap_or(true) {
                continue;
            }
            if !self.ecs_get::<Identity>(oid).map(|identity| is_mobile(identity.kind)).unwrap_or(false) {
                continue;
            }
            if let Some(ox) = self.ecs_get::<Transform>(oid).copied() {
                grid.set_passable(ox.x, ox.y, false);
            }
        }
        grid
    }

    /// 把存活建筑的完整 `Foundation` 再封进通行表（海军改写水面后须重封船厂等）。
    fn reseal_structure_footprints_on_grid(&self, grid: &mut PassGrid) {
        for entity in &self.entities {
            let id = entity.id;
            if self.ecs_get::<Health>(id).map(|h| h.dead).unwrap_or(true) {
                continue;
            }
            if !self.ecs_get::<Identity>(id).map(|i| i.kind == MapEntityKind::Structure).unwrap_or(false) {
                continue;
            }
            let Some(xf) = self.ecs_get::<Transform>(id).copied()
            else {
                continue;
            };
            let foundation = self
                .ecs_get::<Identity>(id)
                .and_then(|i| self.definitions.structures.get_by_id(i.type_id))
                .map(|s| s.foundation.clone())
                .unwrap_or_default();
            let fw = foundation.width.max(1);
            let fh = foundation.height.max(1);
            for dy in 0..fh {
                for dx in 0..fw {
                    let Some(cx) = xf.x.checked_add(dx)
                    else {
                        continue;
                    };
                    let Some(cy) = xf.y.checked_add(dy)
                    else {
                        continue;
                    };
                    grid.set_passable(cx, cy, false);
                }
            }
        }
    }
}

#[doc(hidden)]
pub fn turn_facing_toward(current: &mut u8, desired: u8, step: u8) {
    if *current == desired || step == 0 {
        return;
    }
    let cur = i16::from(*current);
    let want = i16::from(desired);
    let mut delta = (want - cur).rem_euclid(256);
    if delta > 128 {
        delta -= 256;
    }
    let moved = if delta > 0 { delta.min(i16::from(step)) } else { delta.max(-i16::from(step)) };
    *current = (cur + moved).rem_euclid(256) as u8;
}

#[doc(hidden)]
pub fn manhattan(ax: u16, ay: u16, bx: u16, by: u16) -> u32 {
    (i32::from(ax) - i32::from(bx)).unsigned_abs() + (i32::from(ay) - i32::from(by)).unsigned_abs()
}

/// 点到占地矩形（左上锚点 + 宽高）的曼哈顿距离；落在矩形内为 0。
pub fn manhattan_to_footprint(px: u16, py: u16, fx: u16, fy: u16, width: u16, height: u16) -> u32 {
    let width = width.max(1);
    let height = height.max(1);
    let x2 = fx.saturating_add(width.saturating_sub(1));
    let y2 = fy.saturating_add(height.saturating_sub(1));
    let cx = px.clamp(fx, x2);
    let cy = py.clamp(fy, y2);
    manhattan(px, py, cx, cy)
}

/// 是否紧贴占地外沿（曼哈顿距离恰为 1）。
pub fn is_adjacent_to_footprint(px: u16, py: u16, fx: u16, fy: u16, width: u16, height: u16) -> bool {
    manhattan_to_footprint(px, py, fx, fy, width, height) == 1
}

/// 占地外沿上离 `(px,py)` 最近的邻接格（供移动目的地；不查通行）。
pub fn nearest_adjacent_to_footprint(px: u16, py: u16, fx: u16, fy: u16, width: u16, height: u16) -> (u16, u16) {
    let width = width.max(1);
    let height = height.max(1);
    let x1 = i32::from(fx);
    let y1 = i32::from(fy);
    let x2 = x1 + i32::from(width) - 1;
    let y2 = y1 + i32::from(height) - 1;
    let mut best = (fx, fy);
    let mut best_d = u32::MAX;
    for y in (y1 - 1)..=(y2 + 1) {
        for x in (x1 - 1)..=(x2 + 1) {
            if x < 0 || y < 0 {
                continue;
            }
            let xu = x as u16;
            let yu = y as u16;
            if !is_adjacent_to_footprint(xu, yu, fx, fy, width, height) {
                continue;
            }
            let d = manhattan(px, py, xu, yu);
            if d < best_d {
                best_d = d;
                best = (xu, yu);
            }
        }
    }
    best
}

#[doc(hidden)]
pub fn facing_toward(from_x: u16, from_y: u16, to_x: u16, to_y: u16) -> u8 {
    match ((i32::from(to_x) - i32::from(from_x)).signum(), (i32::from(to_y) - i32::from(from_y)).signum()) {
        (1, 0) => 0,
        (1, 1) => 32,
        (0, 1) => 64,
        (-1, 1) => 96,
        (-1, 0) => 128,
        (-1, -1) => 160,
        (0, -1) => 192,
        (1, -1) => 224,
        _ => 0,
    }
}

impl crate::state::BattleState {
    /// 其它存活移动单位是否占用该格（读 ECS）。
    pub fn cell_occupied_by_other(&self, self_i: usize, x: u16, y: u16) -> bool {
        self.entities.iter().enumerate().any(|(j, o)| {
            if j == self_i {
                return false;
            }
            let id = o.id;
            if self.ecs_get::<Health>(id).map(|h| h.dead).unwrap_or(true) {
                return false;
            }
            if !self.ecs_get::<Identity>(id).map(|identity| is_mobile(identity.kind)).unwrap_or(false) {
                return false;
            }
            self.ecs_get::<Transform>(id).map(|t| t.x == x && t.y == y).unwrap_or(false)
        })
    }

    /// 根据 ECS 坐标与目的地计算路径（不写入实体）。
    ///
    /// `Naval=yes` 单位只在水面连通分量内寻路；点到陆地时回落到最近水面格。
    pub fn compute_repath_at(&self, i: usize) -> Vec<(u16, u16)> {
        let id = self.entities[i].id;
        let (Some(tx), Some(ty)) = self.ecs_get::<MovementState>(id).map(|m| (m.destination_x, m.destination_y)).unwrap_or((None, None))
        else {
            return Vec::new();
        };
        let Some(xf) = self.ecs_get::<Transform>(id).copied()
        else {
            return Vec::new();
        };
        let (sx, sy) = (xf.x, xf.y);
        let naval = self.entity_is_naval(id);
        let mut grid = self.prepare_move_grid(i, naval);
        // 打开起点：允许从非法格（如误刷陆地）尝试回到合法地形。
        grid.set_passable(sx, sy, true);
        let (gx, gy) = nearest_free_goal(&grid, sx, sy, tx, ty);
        if !grid.is_passable(gx, gy) {
            // 回落仍不可走：地面单位沿用强制打开终点；海军拒绝穿陆。
            if naval {
                return Vec::new();
            }
            grid.set_passable(gx, gy, true);
        }
        let Some(mut path) = grid.find_path_diag(sx, sy, gx, gy)
        else {
            return Vec::new();
        };
        if path.first() == Some(&(sx, sy)) {
            path.remove(0);
        }
        // 海军：丢弃仍落在非水面的路径点（防止强制打开起点后残留陆格）。
        if naval {
            path.retain(|&(x, y)| self.pass_grid.is_naval_passable(x, y));
        }
        path
    }

    /// 为散开挑选邻近可通行且未被其它机动单位占用的格（确定性，按实体 id 与 tick 盐选）。
    pub(crate) fn pick_scatter_cell(&self, entity_index: usize) -> Option<(u16, u16)> {
        let id = self.entities.get(entity_index)?.id;
        let xf = self.ecs_get::<Transform>(id).copied()?;
        let naval = self.entity_is_naval(id);
        let mut grid = self.prepare_move_grid(entity_index, naval);
        grid.set_passable(xf.x, xf.y, true);
        let salt = id.0.wrapping_add(self.tick);
        for radius in 1_i32..=4 {
            let mut candidates: Vec<(u16, u16)> = Vec::new();
            for dy in -radius..=radius {
                for dx in -radius..=radius {
                    if dx.abs() != radius && dy.abs() != radius {
                        continue;
                    }
                    let x = i32::from(xf.x) + dx;
                    let y = i32::from(xf.y) + dy;
                    if x < 0 || y < 0 {
                        continue;
                    }
                    let (x, y) = (x as u16, y as u16);
                    if !grid.is_passable(x, y) {
                        continue;
                    }
                    if naval && !self.pass_grid.is_naval_passable(x, y) {
                        continue;
                    }
                    if self.cell_occupied_by_other(entity_index, x, y) {
                        continue;
                    }
                    candidates.push((x, y));
                }
            }
            if candidates.is_empty() {
                continue;
            }
            let idx = (salt % candidates.len() as u64) as usize;
            return candidates.get(idx).copied();
        }
        None
    }
}

#[doc(hidden)]
pub fn nearest_free_goal(grid: &PassGrid, sx: u16, sy: u16, tx: u16, ty: u16) -> (u16, u16) {
    if grid.is_passable(tx, ty) {
        return (tx, ty);
    }
    let mut best = None;
    for radius in 1_i32..=8 {
        for dy in -radius..=radius {
            for dx in -radius..=radius {
                if dx.abs() != radius && dy.abs() != radius {
                    continue;
                }
                let (x, y) = (i32::from(tx) + dx, i32::from(ty) + dy);
                if x < 0 || y < 0 || !grid.is_passable(x as u16, y as u16) {
                    continue;
                }
                let key = (((x - i32::from(sx)).pow(2) + (y - i32::from(sy)).pow(2)) as u32, x as u16, y as u16);
                if best.map(|current| key < current).unwrap_or(true) {
                    best = Some(key);
                }
            }
        }
        if best.is_some() {
            break;
        }
    }
    best.map(|(_, x, y)| (x, y)).unwrap_or((tx, ty))
}

/// 弹出路径首格并返回新坐标与朝向（路径写入仍由调用方经 ECS 完成）。
pub fn take_path_step(path: &mut Vec<(u16, u16)>, from_x: u16, from_y: u16) -> Option<(u16, u16, u8)> {
    let (x, y) = path.first().copied()?;
    path.remove(0);
    let facing = facing_toward(from_x, from_y, x, y);
    Some((x, y, facing))
}

impl crate::state::BattleState {
    #[doc(hidden)]
    pub fn advance_movement(&mut self) {
        let n = self.entities.len();
        for i in 0..n {
            let id = self.entities[i].id;
            if self.ecs_get::<Health>(id).map(|h| h.dead).unwrap_or(true) {
                continue;
            }
            if !self.ecs_get::<Identity>(id).map(|identity| is_mobile(identity.kind)).unwrap_or(false) {
                continue;
            }
            let speed = self.ecs_get::<Locomotor>(id).map(|loco| loco.speed).unwrap_or(0);
            if speed == 0 {
                continue;
            }
            // 攻击中且已在射程内：停步开火，不继续挤占目标格。
            if let Some(target_id) = self.ecs_get::<AttackState>(id).and_then(|a| a.target) {
                if self.entity_index(target_id).is_some() {
                    let target_dead = self.ecs_get::<Health>(target_id).map(|h| h.dead).unwrap_or(true);
                    let range = self.ecs_get::<CombatStats>(id).map(|s| s.attack_range).unwrap_or(0);
                    if !target_dead {
                        if let (Some(ax), Some(tx)) = (self.ecs_get::<Transform>(id).copied(), self.ecs_get::<Transform>(target_id).copied()) {
                            if manhattan(ax.x, ax.y, tx.x, tx.y) <= range {
                                let _ = self.with_movement_mut(id, |movement| {
                                    movement.path.clear();
                                });
                                continue;
                            }
                        }
                    }
                }
            }
            let Some(movement) = self.ecs_get::<MovementState>(id).cloned()
            else {
                continue;
            };
            let (Some(tx), Some(ty)) = (movement.destination_x, movement.destination_y)
            else {
                continue;
            };
            let Some(xf) = self.ecs_get::<Transform>(id).copied()
            else {
                continue;
            };
            if xf.x == tx && xf.y == ty {
                let advance = self.with_movement_mut(id, |movement| {
                    movement.path.clear();
                    movement.move_accum = 0;
                    if let Some((nx, ny)) = movement.waypoints.first().copied() {
                        movement.waypoints.remove(0);
                        movement.destination_x = Some(nx);
                        movement.destination_y = Some(ny);
                        true
                    }
                    else {
                        movement.destination_x = None;
                        movement.destination_y = None;
                        false
                    }
                });
                if advance == Some(false) {
                    let _ = self.with_identity_mut(id, |identity| {
                        if identity.mission == Some(ra_types::MissionKind::AttackMove) {
                            identity.mission = None;
                        }
                    });
                }
                if advance == Some(true) {
                    self.repath_entity_at(i);
                }
                continue;
            }
            // 尚无路径时先寻路，便于滑移开始前就朝向下一格。
            if movement.path.is_empty() {
                self.repath_entity_at(i);
            }
            if let Some((nx, ny)) = self.ecs_get::<MovementState>(id).and_then(|m| m.path.first().copied()) {
                let desired = facing_toward(xf.x, xf.y, nx, ny);
                if desired != xf.facing {
                    let _ = self.with_transform_mut(id, |transform| {
                        transform.facing = desired;
                    });
                    self.mark_entity_dirty(id);
                }
            }
            let _ = self.with_movement_mut(id, |movement| {
                movement.move_accum = movement.move_accum.saturating_add(speed);
            });
            // 移动中每 tick 推进 Walk 循环，并标脏以便呈现刷新（勿等跨格才动画面）。
            let _ = self.with_animation_mut(id, |anim| {
                anim.hva_frame = anim.hva_frame.wrapping_add(1);
            });
            self.mark_entity_dirty(id);
            while self.ecs_get::<MovementState>(id).map(|m| m.move_accum).unwrap_or(0) >= crate::state::CELL_MOVE_COST {
                let _ = self.with_movement_mut(id, |movement| {
                    movement.move_accum -= crate::state::CELL_MOVE_COST;
                });
                if self.ecs_get::<MovementState>(id).map(|m| m.path.is_empty()).unwrap_or(true) {
                    self.repath_entity_at(i);
                    if self.ecs_get::<MovementState>(id).map(|m| m.path.is_empty()).unwrap_or(true) {
                        break;
                    }
                }
                let Some((nx, ny)) = self.ecs_get::<MovementState>(id).and_then(|m| m.path.first().copied())
                else {
                    break;
                };
                // 海军硬闸：路径点不得落在非水面（防旧路径 / 强制开起点残留）。
                if self.entity_is_naval(id) && !self.pass_grid.is_naval_passable(nx, ny) {
                    let _ = self.with_movement_mut(id, |movement| {
                        movement.path.clear();
                    });
                    self.repath_entity_at(i);
                    break;
                }
                if self.cell_occupied_by_other(i, nx, ny) {
                    let _ = self.with_movement_mut(id, |movement| {
                        movement.path.clear();
                    });
                    self.repath_entity_at(i);
                    let Some((nx2, ny2)) = self.ecs_get::<MovementState>(id).and_then(|m| m.path.first().copied())
                    else {
                        break;
                    };
                    if self.cell_occupied_by_other(i, nx2, ny2) {
                        break;
                    }
                    if self.entity_is_naval(id) && !self.pass_grid.is_naval_passable(nx2, ny2) {
                        let _ = self.with_movement_mut(id, |movement| {
                            movement.path.clear();
                        });
                        break;
                    }
                }
                let from_x = self.ecs_get::<Transform>(id).map(|t| t.x).unwrap_or(0);
                let from_y = self.ecs_get::<Transform>(id).map(|t| t.y).unwrap_or(0);
                let step = self.with_movement_mut(id, |movement| take_path_step(&mut movement.path, from_x, from_y));
                let Some(Some((x, y, facing))) = step
                else {
                    break;
                };
                let _ = self.with_transform_mut(id, |transform| {
                    transform.facing = facing;
                    transform.x = x;
                    transform.y = y;
                });
                self.mark_entity_dirty(id);
            }
        }
    }
}
