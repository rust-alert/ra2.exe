//! 跟随任务：持续把目的地钉在目标当前格。

use ra_types::MissionKind;

use crate::state::components::{AttackState, Health, Identity, MovementState, Transform};

impl crate::state::BattleState {
    /// 推进 `mission=Follow`：目标存活则追格，否则清跟随并退出任务。
    pub(crate) fn resolve_follow(&mut self) {
        let ids: Vec<_> = self.entities.iter().map(|e| e.id).collect();
        for id in ids {
            let Some(identity) = self.ecs_get::<Identity>(id)
            else {
                continue;
            };
            if identity.mission != Some(MissionKind::Follow) {
                continue;
            }
            if self.ecs_get::<Health>(id).map(|h| h.dead).unwrap_or(true) {
                continue;
            }
            let Some(target_id) = self.ecs_get::<AttackState>(id).and_then(|a| a.follow_target)
            else {
                let _ = self.with_identity_mut(id, |identity| {
                    identity.mission = None;
                });
                continue;
            };
            let target_dead = self.ecs_get::<Health>(target_id).map(|h| h.dead).unwrap_or(true);
            let Some(target_xf) = self.ecs_get::<Transform>(target_id).copied()
            else {
                self.clear_follow(id);
                continue;
            };
            if target_dead {
                self.clear_follow(id);
                continue;
            }
            let Some(self_xf) = self.ecs_get::<Transform>(id).copied()
            else {
                continue;
            };
            // 已与目标同格：清掉目的地，避免 `advance_movement` 清空后再每 tick 重寻路。
            if self_xf.x == target_xf.x && self_xf.y == target_xf.y {
                let needs_clear = self.ecs_get::<MovementState>(id).is_some_and(|m| {
                    m.destination_x.is_some() || m.destination_y.is_some() || !m.path.is_empty() || !m.waypoints.is_empty()
                });
                if needs_clear {
                    let _ = self.with_movement_mut(id, |movement| {
                        movement.destination_x = None;
                        movement.destination_y = None;
                        movement.waypoints.clear();
                        movement.path.clear();
                        movement.move_accum = 0;
                    });
                    self.mark_entity_dirty(id);
                }
                continue;
            }
            let dest = self.ecs_get::<MovementState>(id).map(|m| (m.destination_x, m.destination_y));
            if dest == Some((Some(target_xf.x), Some(target_xf.y))) {
                continue;
            }
            let _ = self.with_movement_mut(id, |movement| {
                movement.destination_x = Some(target_xf.x);
                movement.destination_y = Some(target_xf.y);
                movement.waypoints.clear();
                movement.path.clear();
                movement.move_accum = 0;
            });
            if let Some(idx) = self.entity_index(id) {
                self.repath_entity_at(idx);
            }
            self.mark_entity_dirty(id);
        }
    }

    fn clear_follow(&mut self, id: ra_types::EntityId) {
        let _ = self.with_attack_mut(id, |attack| {
            attack.follow_target = None;
        });
        let _ = self.with_identity_mut(id, |identity| {
            if identity.mission == Some(MissionKind::Follow) {
                identity.mission = None;
            }
        });
        let _ = self.with_movement_mut(id, |movement| {
            movement.destination_x = None;
            movement.destination_y = None;
            movement.waypoints.clear();
            movement.path.clear();
            movement.move_accum = 0;
        });
        self.mark_entity_dirty(id);
    }
}
