//! 单位移动、朝向与路径辅助函数。

use ra_map::{MapEntityKind, PassGrid};

use crate::state::components::{AttackState, CombatStats, Health, Identity, Locomotor, MovementState, Transform};

pub(crate) fn is_mobile(kind: MapEntityKind) -> bool {
    matches!(kind, MapEntityKind::Unit | MapEntityKind::Infantry | MapEntityKind::Aircraft)
}

pub(crate) fn turn_facing_toward(current: &mut u8, desired: u8, step: u8) {
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

pub(crate) fn manhattan(ax: u16, ay: u16, bx: u16, by: u16) -> u32 {
    (i32::from(ax) - i32::from(bx)).unsigned_abs() + (i32::from(ay) - i32::from(by)).unsigned_abs()
}

pub(crate) fn facing_toward(from_x: u16, from_y: u16, to_x: u16, to_y: u16) -> u8 {
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

impl crate::state::MatchState {
    /// 其它存活移动单位是否占用该格（读 ECS）。
    pub(crate) fn cell_occupied_by_other(&self, self_i: usize, x: u16, y: u16) -> bool {
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
    pub(crate) fn compute_repath_at(&self, i: usize) -> Vec<(u16, u16)> {
        let id = self.entities[i].id;
        let (Some(tx), Some(ty)) = self
            .ecs_get::<MovementState>(id)
            .map(|m| (m.destination_x, m.destination_y))
            .unwrap_or((None, None))
        else {
            return Vec::new();
        };
        let Some(xf) = self.ecs_get::<Transform>(id).copied()
        else {
            return Vec::new();
        };
        let (sx, sy) = (xf.x, xf.y);
        let mut grid = self.pass_grid.clone();
        for (j, entity) in self.entities.iter().enumerate() {
            if j == i {
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
        grid.set_passable(sx, sy, true);
        let (gx, gy) = nearest_free_goal(&grid, sx, sy, tx, ty);
        grid.set_passable(gx, gy, true);
        let Some(mut path) = grid.find_path_diag(sx, sy, gx, gy)
        else {
            return Vec::new();
        };
        if path.first() == Some(&(sx, sy)) {
            path.remove(0);
        }
        path
    }
}

fn nearest_free_goal(grid: &PassGrid, sx: u16, sy: u16, tx: u16, ty: u16) -> (u16, u16) {
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
pub(crate) fn take_path_step(path: &mut Vec<(u16, u16)>, from_x: u16, from_y: u16) -> Option<(u16, u16, u8)> {
    let (x, y) = path.first().copied()?;
    path.remove(0);
    let facing = facing_toward(from_x, from_y, x, y);
    Some((x, y, facing))
}

impl crate::state::MatchState {
    pub(crate) fn advance_movement(&mut self) {
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
                        if let (Some(ax), Some(tx)) =
                            (self.ecs_get::<Transform>(id).copied(), self.ecs_get::<Transform>(target_id).copied())
                        {
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
                let _ = self.with_movement_mut(id, |movement| {
                    movement.path.clear();
                });
                continue;
            }
            let _ = self.with_movement_mut(id, |movement| {
                movement.move_accum = movement.move_accum.saturating_add(speed);
            });
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
                let _ = self.with_animation_mut(id, |anim| {
                    anim.hva_frame = anim.hva_frame.wrapping_add(1);
                });
                self.mark_entity_dirty(id);
            }
        }
    }
}
