//! 采矿：矿车站矿格采集、邻接矿场卸货入账。
//!
//! 不再由矿场定时器凭空加钱；仍简化路径（无自动寻矿/返场 AI）。

use std::sync::Arc;

use ra_map::MapEntityKind;

use crate::{
    gameplay::{is_construction_yard, is_harvester, is_power_plant, is_refinery},
    state::{
        ORE_INCOME_PER_TRIP, ORE_TRIP_TICKS,
        components::{Health, Identity, Owner, Transform},
    },
};

impl crate::state::BattleState {
    /// 每个逻辑 tick：空载矿车在矿格采集；满载且邻接存活矿场则卸货加钱。
    pub(crate) fn advance_refinery_income(&mut self) {
        let defs = Arc::clone(&self.definitions);
        let mut deliveries: Vec<(Arc<str>, i32)> = Vec::new();
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
            }
        }
        for (house, amount) in deliveries {
            if let Some(player) = self.players.iter_mut().find(|p| p.house.as_ref() == house.as_ref()) {
                player.funds = player.funds.saturating_add(amount);
            }
        }
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
