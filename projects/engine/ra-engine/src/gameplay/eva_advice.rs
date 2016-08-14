//! 局内 EVA 唠叨（资金不足 SpeakDelay 等）。

use ra_map::MapEntityKind;
use ra_types::ProductionCategory;

use crate::{
    gameplay::definitions_query::is_production_factory,
    state::components::{Health, Identity, Owner},
};

/// 可用资金低于此值时触发资金唠叨（原版阈值）。
pub const FUNDS_NAG_CREDITS: i32 = 100;

/// `[AudioVisual] SpeakDelay`（分钟）× 900 → 逻辑 tick（15fps 基线，暂不按游戏速度归一）。
///
/// 装载期由 adaptor 写入 [`ra_types::RuntimeDefinitions::speak_delay_ticks`]；本函数仅供换算单测。
pub fn speak_delay_ticks_from_minutes(minutes: f64) -> u32 {
    if !(minutes > 0.0) {
        return 0;
    }
    (minutes * 900.0) as u32
}

impl crate::state::BattleState {
    /// 资金不足唠叨：钱包 < 100 且拥有步兵/载具/建筑工厂时，按 `SpeakDelay` 周期排队 `EVA_InsufficientFunds`。
    pub fn tick_eva_funds_nag(&mut self) {
        let delay = self.speak_delay_ticks;
        if delay == 0 {
            return;
        }
        let n = self.players.len();
        for player_index in 0..n {
            if self.players[player_index].eva_funds_nag_ticks > 0 {
                self.players[player_index].eva_funds_nag_ticks -= 1;
            }
            if self.players[player_index].eva_funds_nag_ticks > 0 {
                continue;
            }
            if self.players[player_index].funds >= FUNDS_NAG_CREDITS {
                continue;
            }
            let house = self.players[player_index].house.clone();
            if !self.house_has_funds_nag_factory(house.as_ref()) {
                continue;
            }
            self.push_eva_cue(house.as_ref(), "EVA_InsufficientFunds");
            self.players[player_index].eva_funds_nag_ticks = delay;
        }
    }

    fn house_has_funds_nag_factory(&self, house: &str) -> bool {
        self.entities.iter().any(|e| {
            let id = e.id;
            if self.ecs_get::<Health>(id).map(|h| h.dead).unwrap_or(true) {
                return false;
            }
            if !self.ecs_get::<Owner>(id).map(|o| o.house.as_ref() == house).unwrap_or(false) {
                return false;
            }
            if !self.ecs_get::<Identity>(id).map(|i| i.kind == MapEntityKind::Structure).unwrap_or(false) {
                return false;
            }
            let Some(type_id) = self.ecs_get::<Identity>(id).map(|i| i.type_id.clone())
            else {
                return false;
            };
            if !is_production_factory(&self.definitions, type_id.as_ref()) {
                return false;
            }
            // 与原版 GetFactoryCount 对齐：不计飞行器工厂。
            let cat = self.definitions.structures.get(type_id.as_ref()).and_then(|s| s.production.as_ref()).map(|p| p.category);
            matches!(cat, Some(ProductionCategory::Infantry | ProductionCategory::Vehicle | ProductionCategory::Building))
        })
    }
}
