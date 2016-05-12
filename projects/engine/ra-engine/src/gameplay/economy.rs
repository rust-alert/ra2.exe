//! 采矿：矿车站矿格采集、邻接矿场卸货入账；空闲时自动寻矿 / 返场。

use std::sync::Arc;

use ra_map::MapEntityKind;
use ra_types::EntityId;

use crate::{
    gameplay::{is_construction_yard, is_harvester, is_power_plant, is_refinery},
    state::{
        ORE_INCOME_PER_TRIP, ORE_TRIP_TICKS,
        components::{Health, Identity, MovementState, Owner, Transform},
    },
};

impl crate::state::BattleState {
    /// 每个逻辑 tick：空载矿车在矿格采集；满载且邻接存活矿场则卸货加钱；空闲则自动下发移动目标。
    pub(crate) fn advance_refinery_income(&mut self) {
        let defs = Arc::clone(&self.definitions);
        let mut deliveries: Vec<(Arc<str>, i32)> = Vec::new();
        let mut auto_moves: Vec<(usize, EntityId, u16, u16)> = Vec::new();
        let n = self.entities.len();
        for index in 0..n {
            let id = self.entities[index].id;
            if self.ecs_get::<Health>(id).map(|h| h.dead).unwrap_or(true) {
                continue;
            }
            let Some(identity) = self.ecs_get::<Identity>(id).cloned()
            else {
                continue;
            };
            if !is_harvester(&defs, &identity.type_id) {
                continue;
            }
            let Some(owner) = self.ecs_get::<Owner>(id).map(|o| o.house.clone())
            else {
                continue;
            };
            let Some(xf) = self.ecs_get::<Transform>(id).copied()
            else {
                continue;
            };

            let cargo = self
                .with_harvester_mut(id, |h| h.cargo)
                .unwrap_or(0);

            if cargo == 0 {
                if self.harvestable_ore_at(xf.x, xf.y).is_none() {
                    let _ = self.with_harvester_mut(id, |h| {
                        h.ore_trip_accum = 0;
                    });
                    if self.harvester_movement_idle(id, xf.x, xf.y) {
                        if let Some((tx, ty)) = self.nearest_harvestable_ore(xf.x, xf.y) {
                            auto_moves.push((index, id, tx, ty));
                        }
                    }
                    continue;
                }
                let ready = self
                    .with_harvester_mut(id, |h| {
                        h.ore_trip_accum = h.ore_trip_accum.saturating_add(1);
                        if h.ore_trip_accum >= ORE_TRIP_TICKS {
                            h.ore_trip_accum = 0;
                            true
                        } else {
                            false
                        }
                    })
                    .unwrap_or(false);
                if ready && self.consume_one_ore_at(xf.x, xf.y) {
                    let _ = self.with_harvester_mut(id, |h| {
                        h.cargo = 1;
                    });
                }
                continue;
            }

            if self.house_has_living_refinery_near(owner.as_ref(), xf.x, xf.y) {
                let unloaded = self
                    .with_harvester_mut(id, |h| {
                        if h.cargo > 0 {
                            h.cargo = 0;
                            h.ore_trip_accum = 0;
                            true
                        } else {
                            false
                        }
                    })
                    .unwrap_or(false);
                if unloaded {
                    deliveries.push((owner, ORE_INCOME_PER_TRIP as i32));
                }
            } else if self.harvester_movement_idle(id, xf.x, xf.y) {
                if let Some((tx, ty)) = self.nearest_unload_cell(owner.as_ref(), xf.x, xf.y) {
                    auto_moves.push((index, id, tx, ty));
                }
            }
        }
        for (house, amount) in deliveries {
            if let Some(player) = self.players.iter_mut().find(|p| p.house.as_ref() == house.as_ref()) {
                player.funds = player.funds.saturating_add(amount);
            }
        }
        for (index, id, tx, ty) in auto_moves {
            let _ = self.with_movement_mut(id, |movement| {
                movement.destination_x = Some(tx);
                movement.destination_y = Some(ty);
                movement.path.clear();
                movement.move_accum = 0;
            });
            self.repath_entity_at(index);
        }
    }

    /// 无路径，且无目的地或已抵达目的地（可接自动寻矿/返场）。
    fn harvester_movement_idle(&self, id: EntityId, x: u16, y: u16) -> bool {
        let Some(movement) = self.ecs_get::<MovementState>(id)
        else {
            return true;
        };
        if !movement.path.is_empty() {
            return false;
        }
        match (movement.destination_x, movement.destination_y) {
            (None, None) => true,
            (Some(dx), Some(dy)) => dx == x && dy == y,
            _ => true,
        }
    }

    /// 最近可采矿格（切比雪夫距离）。
    fn nearest_harvestable_ore(&self, from_x: u16, from_y: u16) -> Option<(u16, u16)> {
        let mut best: Option<(u32, u16, u16)> = None;
        for cell in &self.map.overlays {
            if !self.overlay_types.is_harvestable(cell.overlay_id) {
                continue;
            }
            if !self.pass_grid.in_bounds(cell.x, cell.y) || !self.pass_grid.is_passable(cell.x, cell.y) {
                continue;
            }
            let dx = (i32::from(cell.x) - i32::from(from_x)).unsigned_abs();
            let dy = (i32::from(cell.y) - i32::from(from_y)).unsigned_abs();
            let dist = dx.max(dy);
            if best.is_none_or(|(d, _, _)| dist < d) {
                best = Some((dist, cell.x, cell.y));
            }
        }
        best.map(|(_, x, y)| (x, y))
    }

    /// 最近同阵营存活矿场的可走邻格（含矿场同格若可走）。
    fn nearest_unload_cell(&self, house: &str, from_x: u16, from_y: u16) -> Option<(u16, u16)> {
        let defs = &self.definitions;
        let mut best: Option<(u32, u16, u16)> = None;
        for e in &self.entities {
            let id = e.id;
            if self.ecs_get::<Health>(id).map(|h| h.dead).unwrap_or(true) {
                continue;
            }
            if self.ecs_get::<Owner>(id).is_none_or(|o| o.house.as_ref() != house) {
                continue;
            }
            let Some(identity) = self.ecs_get::<Identity>(id)
            else {
                continue;
            };
            if identity.kind != MapEntityKind::Structure || !is_refinery(defs, &identity.type_id) {
                continue;
            }
            let Some(rxf) = self.ecs_get::<Transform>(id).copied()
            else {
                continue;
            };
            for (tx, ty) in self.refinery_approach_cells(rxf.x, rxf.y) {
                let dx = (i32::from(tx) - i32::from(from_x)).unsigned_abs();
                let dy = (i32::from(ty) - i32::from(from_y)).unsigned_abs();
                let dist = dx.max(dy);
                if best.is_none_or(|(d, _, _)| dist < d) {
                    best = Some((dist, tx, ty));
                }
            }
        }
        best.map(|(_, x, y)| (x, y))
    }

    fn refinery_approach_cells(&self, rx: u16, ry: u16) -> Vec<(u16, u16)> {
        const DELTAS: [(i32, i32); 9] = [
            (0, 0),
            (1, 0),
            (0, 1),
            (-1, 0),
            (0, -1),
            (1, 1),
            (-1, 1),
            (-1, -1),
            (1, -1),
        ];
        let mut out = Vec::new();
        for (dx, dy) in DELTAS {
            let x = i32::from(rx) + dx;
            let y = i32::from(ry) + dy;
            if x < 0 || y < 0 {
                continue;
            }
            let (x, y) = (x as u16, y as u16);
            if self.pass_grid.in_bounds(x, y) && self.pass_grid.is_passable(x, y) {
                out.push((x, y));
            }
        }
        out
    }

    /// 扣减一格可采矿密度；密度归零则移除该 overlay。
    fn consume_one_ore_at(&mut self, x: u16, y: u16) -> bool {
        let Some(index) = self.map.overlays.iter().position(|cell| {
            cell.x == x && cell.y == y && self.overlay_types.is_harvestable(cell.overlay_id)
        })
        else {
            return false;
        };
        if self.map.overlays[index].data > 0 {
            self.map.overlays[index].data -= 1;
        }
        if self.map.overlays[index].data == 0 {
            self.map.overlays.remove(index);
        }
        self.overlay_paint_dirty.push((x, y));
        true
    }

    /// 同阵营存活矿场是否在切比雪夫距离 ≤ 1（含同格）。
    fn house_has_living_refinery_near(&self, house: &str, x: u16, y: u16) -> bool {
        let defs = &self.definitions;
        self.entities.iter().any(|e| {
            let id = e.id;
            if self.ecs_get::<Health>(id).map(|h| h.dead).unwrap_or(true) {
                return false;
            }
            if self.ecs_get::<Owner>(id).is_none_or(|o| o.house.as_ref() != house) {
                return false;
            }
            let Some(identity) = self.ecs_get::<Identity>(id)
            else {
                return false;
            };
            if identity.kind != MapEntityKind::Structure || !is_refinery(defs, &identity.type_id) {
                return false;
            }
            let Some(rxf) = self.ecs_get::<Transform>(id)
            else {
                return false;
            };
            let dx = (rxf.x as i32 - x as i32).unsigned_abs();
            let dy = (rxf.y as i32 - y as i32).unsigned_abs();
            dx.max(dy) <= 1
        })
    }

    pub(crate) fn house_has_living_yard(&self, house: &str) -> bool {
        self.entities.iter().any(|e| {
            let id = e.id;
            !self.ecs_get::<Health>(id).map(|h| h.dead).unwrap_or(true)
                && self.ecs_get::<Owner>(id).map(|o| o.house.as_ref() == house).unwrap_or(false)
                && self.ecs_get::<Identity>(id).map(|i| i.kind == MapEntityKind::Structure).unwrap_or(false)
                && self
                    .ecs_get::<Identity>(id)
                    .map(|i| is_construction_yard(&self.definitions, &i.type_id))
                    .unwrap_or(false)
        })
    }

    pub(crate) fn house_has_living_power(&self, house: &str) -> bool {
        self.entities.iter().any(|e| {
            let id = e.id;
            !self.ecs_get::<Health>(id).map(|h| h.dead).unwrap_or(true)
                && self.ecs_get::<Owner>(id).map(|o| o.house.as_ref() == house).unwrap_or(false)
                && self.ecs_get::<Identity>(id).map(|i| i.kind == MapEntityKind::Structure).unwrap_or(false)
                && self
                    .ecs_get::<Identity>(id)
                    .map(|i| is_power_plant(&self.definitions, &i.type_id))
                    .unwrap_or(false)
        })
    }
}
