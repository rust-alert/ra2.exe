//! 矿场收入与基地供电查询。
//!
//! **明示原型**：存活矿场按固定 tick 直接加钱，不经过矿车、矿量、采集与返场。
//! 不能计作原版采矿完成。生产时长见固定 `PRODUCE_TICKS`，同样是原型节奏。

use std::sync::Arc;

use ra_map::MapEntityKind;

use crate::{
    gameplay::{is_construction_yard, is_power_plant, is_refinery},
    state::{ORE_INCOME_PER_TRIP, ORE_TRIP_TICKS},
};

impl crate::state::MatchState {
    pub(crate) fn advance_refinery_income(&mut self) {
        let defs = Arc::clone(&self.definitions);
        let mut credits: Vec<(Arc<str>, i32)> = Vec::new();
        for e in &mut self.entities {
            if e.dead || !is_refinery(&defs, &e.type_id) {
                continue;
            }
            e.ore_trip_accum = e.ore_trip_accum.saturating_add(1);
            if e.ore_trip_accum >= ORE_TRIP_TICKS {
                e.ore_trip_accum = 0;
                credits.push((e.owner.clone(), ORE_INCOME_PER_TRIP as i32));
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
            !e.dead && e.owner.as_ref() == house && e.kind == MapEntityKind::Structure && is_construction_yard(&self.definitions, &e.type_id)
        })
    }

    pub(crate) fn house_has_living_power(&self, house: &str) -> bool {
        self.entities.iter().any(|e| {
            !e.dead && e.owner.as_ref() == house && e.kind == MapEntityKind::Structure && is_power_plant(&self.definitions, &e.type_id)
        })
    }
}
