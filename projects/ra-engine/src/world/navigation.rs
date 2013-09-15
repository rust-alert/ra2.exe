//! 单位移动、朝向与路径辅助函数。

use ra_map::{MapEntityKind, PassGrid};

use super::WorldEntity;

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

pub(crate) fn cell_occupied_by_other(entities: &[WorldEntity], self_i: usize, x: u16, y: u16) -> bool {
    entities.iter().enumerate().any(|(j, o)| j != self_i && !o.dead && is_mobile(o.kind) && o.x == x && o.y == y)
}

pub(crate) fn repath_at(entities: &mut [WorldEntity], i: usize, grid: &PassGrid) {
    entities[i].path.clear();
    let (Some(tx), Some(ty)) = (entities[i].target_x, entities[i].target_y)
    else {
        return;
    };
    let (sx, sy) = (entities[i].x, entities[i].y);
    let mut grid = grid.clone();
    for (j, entity) in entities.iter().enumerate() {
        if j != i && !entity.dead && is_mobile(entity.kind) {
            grid.set_passable(entity.x, entity.y, false);
        }
    }
    grid.set_passable(sx, sy, true);
    let (gx, gy) = nearest_free_goal(&grid, sx, sy, tx, ty);
    grid.set_passable(gx, gy, true);
    let Some(mut path) = grid.find_path_diag(sx, sy, gx, gy)
    else {
        return;
    };
    if path.first() == Some(&(sx, sy)) {
        path.remove(0);
    }
    entities[i].path = path;
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

pub(crate) fn step_along_path(entity: &mut WorldEntity) -> bool {
    let Some((x, y)) = entity.path.first().copied()
    else {
        return false;
    };
    entity.path.remove(0);
    entity.facing = facing_toward(entity.x, entity.y, x, y);
    entity.x = x;
    entity.y = y;
    true
}

impl super::World {
    pub(crate) fn advance_movement(&mut self) {
        let n = self.entities.len();
        for i in 0..n {
            if self.entities[i].dead || !is_mobile(self.entities[i].kind) || self.entities[i].speed == 0 {
                continue;
            }
            // 攻击中且已在射程内：停步开火，不继续挤占目标格。
            if let Some(ti) = self.entities[i].attack_target {
                if ti < n
                    && !self.entities[ti].dead
                    && manhattan(self.entities[i].x, self.entities[i].y, self.entities[ti].x, self.entities[ti].y)
                        <= self.entities[i].attack_range
                {
                    self.entities[i].path.clear();
                    continue;
                }
            }
            let (Some(tx), Some(ty)) = (self.entities[i].target_x, self.entities[i].target_y)
            else {
                continue;
            };
            if self.entities[i].x == tx && self.entities[i].y == ty {
                self.entities[i].path.clear();
                continue;
            }
            self.entities[i].move_accum = self.entities[i].move_accum.saturating_add(self.entities[i].speed);
            while self.entities[i].move_accum >= super::CELL_MOVE_COST {
                self.entities[i].move_accum -= super::CELL_MOVE_COST;
                if self.entities[i].path.is_empty() {
                    repath_at(&mut self.entities, i, &self.pass_grid);
                    if self.entities[i].path.is_empty() {
                        break;
                    }
                }
                let Some((nx, ny)) = self.entities[i].path.first().copied()
                else {
                    break;
                };
                if cell_occupied_by_other(&self.entities, i, nx, ny) {
                    self.entities[i].path.clear();
                    repath_at(&mut self.entities, i, &self.pass_grid);
                    let Some((nx2, ny2)) = self.entities[i].path.first().copied()
                    else {
                        break;
                    };
                    if cell_occupied_by_other(&self.entities, i, nx2, ny2) {
                        break;
                    }
                }
                if !step_along_path(&mut self.entities[i]) {
                    break;
                }
                self.entities[i].hva_frame = self.entities[i].hva_frame.wrapping_add(1);
            }
        }
    }
}
