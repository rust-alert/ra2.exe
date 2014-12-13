//! 矿场收入与基地供电查询。
//!
//! **明示原型**：存活矿场按固定 tick 直接加钱，不经过矿车、矿量、采集与返场。
//! 不能计作原版采矿完成。生产时长见固定 `PRODUCE_TICKS`，同样是原型节奏。

use std::sync::Arc;

use ra_map::MapEntityKind;

use crate::{
    gameplay::{is_construction_yard, is_power_plant, is_refinery},
    state::{
        ORE_INCOME_PER_TRIP, ORE_TRIP_TICKS,
        components::{Health, Identity, Owner},
    },
};

impl crate::state::MatchState {
    pub(crate) fn advance_refinery_income(&mut self) {
        let defs = Arc::clone(&self.definitions);
        let mut credits: Vec<(Arc<str>, i32)> = Vec::new();
        let n = self.entities.len();
        for index in 0..n {
            let id = self.entities[index].id;
            if self.ecs_get::<Health>(id).map(|h| h.dead).unwrap_or(true) {
                continue;
            }
            let Some(type_id) = self.ecs_get::<Identity>(id).map(|i| i.type_id.clone())
            else {
                continue;
            };
            if !is_refinery(&defs, &type_id) {
                continue;
            }
            let Some(owner) = self.ecs_get::<Owner>(id).map(|o| o.house.clone())
            else {
                continue;
            };
            let paid = self
                .with_harvester_mut(id, |harvester| {
                    harvester.ore_trip_accum = harvester.ore_trip_accum.saturating_add(1);
                    if harvester.ore_trip_accum >= ORE_TRIP_TICKS {
                        harvester.ore_trip_accum = 0;
                        true
                    }
                    else {
                        false
                    }
                })
                .unwrap_or(false);
            if paid {
                credits.push((owner, ORE_INCOME_PER_TRIP as i32));
            }
        }
        for (house, amount) in credits {
            if let Some(player) = self.players.iter_mut().find(|p| p.house.as_ref() == house.as_ref()) {
                player.funds = player.funds.saturating_add(amount);
            }
        }
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
