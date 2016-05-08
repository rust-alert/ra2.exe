//! 工程师占领：邻接敌方可俘建筑后换房主并移除工程师。

use ra_map::MapEntityKind;
use ra_types::EntityId;
use std::sync::Arc;

use crate::{
    gameplay::{building_power, is_capturable},
    spatial::manhattan,
    state::components::{AttackState, Health, Identity, Owner, Transform},
};

impl crate::state::BattleState {
    /// 工程师邻接可俘建筑时换房主、重算电力，并阵亡工程师。
    pub(crate) fn resolve_capture_building(&mut self) {
        let engineers: Vec<EntityId> = self
            .entities
            .iter()
            .filter_map(|e| {
                let id = e.id;
                if self.ecs_get::<Health>(id).map(|h| h.dead).unwrap_or(true) {
                    return None;
                }
                let attack = self.ecs_get::<AttackState>(id)?;
                attack.capture_target?;
                Some(id)
            })
            .collect();

        for engineer_id in engineers {
            let Some(building_id) = self
                .ecs_get::<AttackState>(engineer_id)
                .and_then(|a| a.capture_target)
            else {
                continue;
            };
            if self.ecs_get::<Health>(building_id).map(|h| h.dead).unwrap_or(true)
                || self.entity_index(building_id).is_none()
            {
                let _ = self.with_attack_mut(engineer_id, |attack| {
                    attack.capture_target = None;
                });
                continue;
            }
            if !self
                .ecs_get::<Identity>(building_id)
                .map(|i| i.kind == MapEntityKind::Structure)
                .unwrap_or(false)
            {
                let _ = self.with_attack_mut(engineer_id, |attack| {
                    attack.capture_target = None;
                });
                continue;
            }
            let Some(building_type) = self.ecs_get::<Identity>(building_id).map(|i| i.type_id.clone())
            else {
                continue;
            };
            if !is_capturable(&self.definitions, building_type.as_ref()) {
                let _ = self.with_attack_mut(engineer_id, |attack| {
                    attack.capture_target = None;
                });
                continue;
            }
            let engineer_house = self.ecs_get::<Owner>(engineer_id).map(|o| o.house.clone());
            let building_house = self.ecs_get::<Owner>(building_id).map(|o| o.house.clone());
            let Some(engineer_house) = engineer_house
            else {
                continue;
            };
            let Some(building_house) = building_house
            else {
                continue;
            };
            if engineer_house == building_house {
                let _ = self.with_attack_mut(engineer_id, |attack| {
                    attack.capture_target = None;
                });
                continue;
            }
            let Some(engineer_xf) = self.ecs_get::<Transform>(engineer_id).copied()
            else {
                continue;
            };
            let Some(building_xf) = self.ecs_get::<Transform>(building_id).copied()
            else {
                continue;
            };
            if manhattan(engineer_xf.x, engineer_xf.y, building_xf.x, building_xf.y) > 1 {
                let _ = self.with_movement_mut(engineer_id, |movement| {
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

            self.transfer_structure_owner(
                building_id,
                building_type.as_ref(),
                building_house.as_ref(),
                engineer_house.as_ref(),
            );
            let capturer_eva = if self
                .definitions
                .prerequisite_groups
                .is_tech_building(building_type.as_ref())
            {
                "EVA_TechBuildingCaptured"
            } else {
                "EVA_BuildingCaptured"
            };
            self.push_eva_cue(engineer_house.as_ref(), capturer_eva);
            self.push_eva_cue(building_house.as_ref(), "EVA_BuildingCaptured");
            self.finish_capturing_engineer(engineer_id);
        }
    }

    fn transfer_structure_owner(
        &mut self,
        building_id: EntityId,
        type_id: &str,
        from_house: &str,
        to_house: &str,
    ) {
        self.revoke_structure_power(from_house, type_id);
        let new_house = Arc::<str>::from(to_house);
        let _ = self.with_owner_mut(building_id, |owner| {
            owner.house = new_house;
        });
        self.grant_structure_power(to_house, type_id);
        // 清空生产队列，避免换房后继续产出旧方单位。
        let _ = self.with_production_mut(building_id, |queue| {
            queue.item = None;
        });
        self.mark_entity_dirty(building_id);
    }

    pub(crate) fn grant_structure_power(&mut self, house: &str, type_id: &str) {
        let power = building_power(&self.definitions, type_id);
        let Some(player) = self.players.iter_mut().find(|p| p.house.as_ref() == house)
        else {
            return;
        };
        player.power_output = player.power_output.saturating_add(power.output);
        player.power_drain = player.power_drain.saturating_add(power.drain);
    }

    fn finish_capturing_engineer(&mut self, engineer_id: EntityId) {
        let Some(index) = self.entity_index(engineer_id)
        else {
            return;
        };
        let max_hp = self.ecs_get::<Health>(engineer_id).map(|h| h.maximum).unwrap_or(1);
        self.apply_damage(index, max_hp.saturating_mul(2).max(1));
        let _ = self.with_attack_mut(engineer_id, |attack| {
            attack.capture_target = None;
            attack.infiltrate_target = None;
            attack.target = None;
        });
    }
}
