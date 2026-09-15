//! 间谍渗透：邻接敌方建筑后结算效果并移除间谍。

use ra_map::MapEntityKind;
use ra_types::{EntityId, InfiltrationEffect, StolenTechKind};

use crate::{
    spatial::{is_adjacent_to_footprint, nearest_adjacent_to_footprint},
    state::components::{AttackState, Health, Identity, Owner, Transform},
};

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
            let Some(building_id) = self.ecs_get::<AttackState>(agent_id).and_then(|a| a.infiltrate_target)
            else {
                continue;
            };
            if self.ecs_get::<Health>(building_id).map(|h| h.dead).unwrap_or(true) || self.entity_index(building_id).is_none() {
                let _ = self.with_attack_mut(agent_id, |attack| {
                    attack.infiltrate_target = None;
                });
                continue;
            }
            if !self.ecs_get::<Identity>(building_id).map(|i| i.kind == MapEntityKind::Structure).unwrap_or(false) {
                let _ = self.with_attack_mut(agent_id, |attack| {
                    attack.infiltrate_target = None;
                });
                continue;
            }
            let agent_house =
                self.ecs_get::<Owner>(agent_id).map(|o| std::sync::Arc::<str>::from(crate::gameplay::house_key_of(&self.definitions, o.house)));
            let building_house = self
                .ecs_get::<Owner>(building_id)
                .map(|o| std::sync::Arc::<str>::from(crate::gameplay::house_key_of(&self.definitions, o.house)));
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
            let building_type_for_foundation = self.ecs_get::<Identity>(building_id).map(|i| i.type_id);
            let foundation = building_type_for_foundation
                .and_then(|t| self.definitions.structures.get_by_id(t))
                .map(|s| s.foundation.clone())
                .unwrap_or_default();
            if !is_adjacent_to_footprint(agent_xf.x, agent_xf.y, building_xf.x, building_xf.y, foundation.width, foundation.height) {
                let (ax, ay) =
                    nearest_adjacent_to_footprint(agent_xf.x, agent_xf.y, building_xf.x, building_xf.y, foundation.width, foundation.height);
                let _ = self.with_movement_mut(agent_id, |movement| {
                    if movement.destination_x != Some(ax) || movement.destination_y != Some(ay) {
                        movement.destination_x = Some(ax);
                        movement.destination_y = Some(ay);
                        movement.path.clear();
                        movement.move_accum = 0;
                    }
                });
                continue;
            }

            let building_type = building_type_for_foundation;
            if let Some(type_id) = building_type {
                let (agent_eva, victim_eva) = self.apply_infiltrate_effect(agent_house.as_ref(), building_house.as_ref(), type_id);
                self.push_eva_cue(agent_house.as_ref(), agent_eva);
                if let Some(victim_event) = victim_eva {
                    self.push_eva_cue(building_house.as_ref(), victim_event);
                }
            }
            self.finish_infiltrating_agent(agent_id);
        }
    }

    /// 结算渗透效果，并返回（行动方 EVA，受害方可选 EVA）。
    ///
    /// 效果类别只读冻结 [`ra_types::InfiltrationProfile`]，不在此按电厂／矿场等启发式分支。
    fn apply_infiltrate_effect(
        &mut self,
        agent_house: &str,
        victim_house: &str,
        building_type: ra_types::TypeId,
    ) -> (&'static str, Option<&'static str>) {
        let blackout = self.definitions.infiltration.power_blackout_ticks;
        let steal_cap = self.definitions.infiltration.refinery_steal_funds;
        let effect = self
            .definitions
            .structures
            .get_by_id(building_type)
            .map(|s| s.infiltration.effect)
            .unwrap_or(InfiltrationEffect::Generic);
        match effect {
            InfiltrationEffect::PowerBlackout => {
                if let Some(player) = self.players.iter_mut().find(|p| p.house.eq_ignore_ascii_case(victim_house)) {
                    player.power_blackout_ticks = blackout.max(player.power_blackout_ticks);
                }
                ("EVA_BuildingInfiltratedPowerSabotaged", Some("EVA_PowerSabotaged"))
            }
            InfiltrationEffect::StealFunds => {
                let stolen = self
                    .players
                    .iter()
                    .find(|p| p.house.eq_ignore_ascii_case(victim_house))
                    .map(|p| p.funds.min(steal_cap).max(0))
                    .unwrap_or(0);
                if stolen > 0 {
                    if let Some(victim) = self.players.iter_mut().find(|p| p.house.eq_ignore_ascii_case(victim_house)) {
                        victim.funds -= stolen;
                    }
                    if let Some(agent) = self.players.iter_mut().find(|p| p.house.eq_ignore_ascii_case(agent_house)) {
                        agent.funds = agent.funds.saturating_add(stolen);
                    }
                }
                ("EVA_CashStolen", Some("EVA_BuildingInfiltrated"))
            }
            InfiltrationEffect::PromoteInfantry => {
                if let Some(player) = self.players.iter_mut().find(|p| p.house.eq_ignore_ascii_case(agent_house)) {
                    player.promoted_infantry = true;
                }
                ("EVA_BuildingInfiltrated", None)
            }
            InfiltrationEffect::PromoteVehicle => {
                if let Some(player) = self.players.iter_mut().find(|p| p.house.eq_ignore_ascii_case(agent_house)) {
                    player.promoted_vehicle = true;
                }
                ("EVA_BuildingInfiltrated", None)
            }
            InfiltrationEffect::StealTech => {
                if let Some(kind) =
                    crate::gameplay::house_id_of(&self.definitions, victim_house).and_then(|id| self.definitions.stolen_tech_by_house.get(id))
                {
                    if let Some(player) = self.players.iter_mut().find(|p| p.house.eq_ignore_ascii_case(agent_house)) {
                        match kind {
                            StolenTechKind::Allied => player.stolen_allied_tech = true,
                            StolenTechKind::Soviet => player.stolen_soviet_tech = true,
                            StolenTechKind::Third => player.stolen_third_tech = true,
                        }
                    }
                }
                ("EVA_NewTechnologyAcquired", Some("EVA_BuildingInfiltrated"))
            }
            InfiltrationEffect::Generic => ("EVA_BuildingInfiltrated", Some("EVA_BuildingInfiltrated")),
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
