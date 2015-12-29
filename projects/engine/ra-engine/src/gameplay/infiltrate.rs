//! 间谍渗透：邻接敌方建筑后结算效果并移除间谍。

use ra_map::MapEntityKind;
use ra_types::{EntityId, ProductionCategory, StolenTechKind};

use crate::{
    gameplay::{is_power_plant, is_refinery},
    spatial::manhattan,
    state::components::{AttackState, Health, Identity, Owner, Transform},
};

/// 渗透电厂后的断电时长（tick）。
pub(crate) const POWER_BLACKOUT_TICKS: u32 = 300;
/// 渗透矿场时最多转走的资金。
pub(crate) const REFINERY_STEAL_FUNDS: i32 = 5_000;

impl crate::state::BattleState {
    /// 推进玩家断电计时。
    pub(crate) fn tick_power_blackouts(&mut self) {
        for player in &mut self.players {
            if player.power_blackout_ticks > 0 {
                player.power_blackout_ticks -= 1;
            }
        }
    }

    /// 间谍邻接渗透目标建筑时结算效果并阵亡。
    pub(crate) fn resolve_infiltrate(&mut self) {
        let agents: Vec<EntityId> = self
            .entities
            .iter()
            .filter_map(|e| {
                let id = e.id;
                if self.ecs_get::<Health>(id).map(|h| h.dead).unwrap_or(true) {
                    return None;
                }
                let attack = self.ecs_get::<AttackState>(id)?;
                attack.infiltrate_target?;
                Some(id)
            })
            .collect();

        for agent_id in agents {
            let Some(building_id) = self
                .ecs_get::<AttackState>(agent_id)
                .and_then(|a| a.infiltrate_target)
            else {
                continue;
            };
            if self.ecs_get::<Health>(building_id).map(|h| h.dead).unwrap_or(true)
                || self.entity_index(building_id).is_none()
            {
                let _ = self.with_attack_mut(agent_id, |attack| {
                    attack.infiltrate_target = None;
                });
                continue;
            }
            if !self
                .ecs_get::<Identity>(building_id)
                .map(|i| i.kind == MapEntityKind::Structure)
                .unwrap_or(false)
            {
                let _ = self.with_attack_mut(agent_id, |attack| {
                    attack.infiltrate_target = None;
                });
                continue;
            }
            let agent_house = self.ecs_get::<Owner>(agent_id).map(|o| o.house.clone());
            let building_house = self.ecs_get::<Owner>(building_id).map(|o| o.house.clone());
            let Some(agent_house) = agent_house
            else {
                continue;
            };
            let Some(building_house) = building_house
            else {
                continue;
            };
            if agent_house == building_house {
                let _ = self.with_attack_mut(agent_id, |attack| {
                    attack.infiltrate_target = None;
                });
                continue;
            }
            let Some(agent_xf) = self.ecs_get::<Transform>(agent_id).copied()
            else {
                continue;
            };
            let Some(building_xf) = self.ecs_get::<Transform>(building_id).copied()
            else {
                continue;
            };
            if manhattan(agent_xf.x, agent_xf.y, building_xf.x, building_xf.y) > 1 {
                // 仍追建筑格（建筑不移动，命令已设目的地；若被清掉则补钉）。
                let _ = self.with_movement_mut(agent_id, |movement| {
                    if movement.destination_x != Some(building_xf.x)
                        || movement.destination_y != Some(building_xf.y)
                    {
                        movement.destination_x = Some(building_xf.x);
                        movement.destination_y = Some(building_xf.y);
                        movement.path.clear();
                        movement.move_accum = 0;
                    }
                });
                continue;
            }

            let building_type = self
                .ecs_get::<Identity>(building_id)
                .map(|i| i.type_id.clone());
            if let Some(type_id) = building_type {
                self.apply_infiltrate_effect(agent_house.as_ref(), building_house.as_ref(), type_id.as_ref());
            }
            self.finish_infiltrating_agent(agent_id);
        }
    }

    fn apply_infiltrate_effect(&mut self, agent_house: &str, victim_house: &str, building_type: &str) {
        if is_power_plant(&self.definitions, building_type) {
            if let Some(player) = self.players.iter_mut().find(|p| p.house.as_ref() == victim_house) {
                player.power_blackout_ticks = POWER_BLACKOUT_TICKS.max(player.power_blackout_ticks);
            }
            return;
        }
        if is_refinery(&self.definitions, building_type) {
            let stolen = self
                .players
                .iter()
                .find(|p| p.house.as_ref() == victim_house)
                .map(|p| p.funds.min(REFINERY_STEAL_FUNDS).max(0))
                .unwrap_or(0);
            if stolen > 0 {
                if let Some(victim) = self.players.iter_mut().find(|p| p.house.as_ref() == victim_house) {
                    victim.funds -= stolen;
                }
                if let Some(agent) = self.players.iter_mut().find(|p| p.house.as_ref() == agent_house) {
                    agent.funds = agent.funds.saturating_add(stolen);
                }
            }
            return;
        }
        if let Some(prod) = self
            .definitions
            .structures
            .get(building_type)
            .and_then(|s| s.production.as_ref())
        {
            if let Some(player) = self.players.iter_mut().find(|p| p.house.as_ref() == agent_house) {
                match prod.category {
                    ProductionCategory::Infantry => player.promoted_infantry = true,
                    ProductionCategory::Vehicle => player.promoted_vehicle = true,
                    ProductionCategory::Aircraft | ProductionCategory::Building => {}
                }
            }
            return;
        }
        if self.definitions.prerequisite_groups.is_tech_building(building_type) {
            if let Some(kind) = self.definitions.stolen_tech_by_house.get(victim_house) {
                if let Some(player) = self.players.iter_mut().find(|p| p.house.as_ref() == agent_house) {
                    match kind {
                        StolenTechKind::Allied => player.stolen_allied_tech = true,
                        StolenTechKind::Soviet => player.stolen_soviet_tech = true,
                        StolenTechKind::Third => player.stolen_third_tech = true,
                    }
                }
            }
        }
    }

    fn finish_infiltrating_agent(&mut self, agent_id: EntityId) {
        let Some(index) = self.entity_index(agent_id)
        else {
            return;
        };
        let max_hp = self.ecs_get::<Health>(agent_id).map(|h| h.maximum).unwrap_or(1);
        self.apply_damage(index, max_hp.saturating_mul(2).max(1));
        let _ = self.with_attack_mut(agent_id, |attack| {
            attack.infiltrate_target = None;
            attack.target = None;
        });
    }
}
