//! 攻击、伤害、受击闪白与炮塔转向。

use ra_map::MapEntityKind;

use crate::{
    gameplay::building_power,
    spatial::{facing_toward, is_mobile, manhattan, turn_facing_toward},
    state::{
        HIT_FLASH_TICKS, TURRET_TURN_STEP,
        components::{AnimationState, AttackState, CombatStats, Health, Identity, Transform},
    },
};
use ra_assets::armor_index;

impl crate::state::BattleState {
    pub(crate) fn resolve_combat(&mut self) {
        let n = self.entities.len();
        let mut damage_events: Vec<(usize, u32)> = Vec::new();
        for i in 0..n {
            let attacker_id = self.entities[i].id;
            let Some(health) = self.ecs_get::<Health>(attacker_id)
            else {
                continue;
            };
            if health.dead {
                continue;
            }
            let Some(identity) = self.ecs_get::<Identity>(attacker_id)
            else {
                continue;
            };
            if !is_mobile(identity.kind) {
                continue;
            }
            let Some(attack) = self.ecs_get::<AttackState>(attacker_id)
            else {
                continue;
            };
            // 渗透中的间谍不开火。
            if attack.infiltrate_target.is_some() {
                continue;
            }
            let Some(target_id) = attack.target
            else {
                continue;
            };
            let Some(ti) = self.entity_index(target_id)
            else {
                let _ = self.with_attack_mut(attacker_id, |attack| {
                    attack.target = None;
                });
                continue;
            };
            let target_dead = self.ecs_get::<Health>(target_id).map(|h| h.dead).unwrap_or(true);
            if target_dead || ti == i {
                let _ = self.with_attack_mut(attacker_id, |attack| {
                    attack.target = None;
                });
                continue;
            }
            // 追击：把移动目标钉在敌人当前格。
            let Some(target_xf) = self.ecs_get::<Transform>(target_id).copied()
            else {
                continue;
            };
            let _ = self.with_movement_mut(attacker_id, |movement| {
                movement.destination_x = Some(target_xf.x);
                movement.destination_y = Some(target_xf.y);
            });

            let cooldown = self.ecs_get::<AttackState>(attacker_id).map(|a| a.cooldown).unwrap_or(0);
            if cooldown > 0 {
                let _ = self.with_attack_mut(attacker_id, |attack| {
                    attack.cooldown = attack.cooldown.saturating_sub(1);
                });
                continue;
            }
            let Some(attacker_xf) = self.ecs_get::<Transform>(attacker_id).copied()
            else {
                continue;
            };
            let Some(stats) = self.ecs_get::<CombatStats>(attacker_id).cloned()
            else {
                continue;
            };
            let Some(target_stats) = self.ecs_get::<CombatStats>(target_id)
            else {
                continue;
            };
            let dist = manhattan(attacker_xf.x, attacker_xf.y, target_xf.x, target_xf.y);
            if dist <= stats.attack_range {
                let dmg = scale_damage(stats.attack_damage, &stats.attack_verses, target_stats.armor.as_str());
                damage_events.push((ti, dmg));
                let cooldown_max = stats.attack_cooldown_max;
                let _ = self.with_attack_mut(attacker_id, |attack| {
                    attack.cooldown = cooldown_max;
                });
            }
        }
        for (ti, dmg) in damage_events {
            self.apply_damage(ti, dmg);
        }
    }

    pub(crate) fn tick_hit_flash(&mut self) {
        let ids: Vec<_> = self
            .entities
            .iter()
            .filter_map(|e| {
                let anim = self.ecs_get::<AnimationState>(e.id)?;
                (anim.hit_flash > 0).then_some(e.id)
            })
            .collect();
        for id in ids {
            let _ = self.with_animation_mut(id, |anim| {
                if anim.hit_flash > 0 {
                    anim.hit_flash -= 1;
                }
            });
            self.mark_entity_dirty(id);
        }
    }

    pub(crate) fn apply_damage(&mut self, index: usize, amount: u32) {
        if index >= self.entities.len() || amount == 0 {
            return;
        }
        let dirty_id = self.entities[index].id;
        if self.ecs_get::<Health>(dirty_id).map(|h| h.dead).unwrap_or(true) {
            return;
        }
        let house = self.ecs_get::<crate::state::components::Owner>(dirty_id).map(|o| o.house.clone());
        let kind = self.ecs_get::<Identity>(dirty_id).map(|i| i.kind);
        let type_id = self.ecs_get::<Identity>(dirty_id).map(|i| i.type_id.clone());
        let cell = self.ecs_get::<Transform>(dirty_id).map(|t| (t.x, t.y));
        let (Some(house), Some(kind), Some(type_id), Some((x, y))) = (house, kind, type_id, cell)
        else {
            return;
        };
        let killed = self
            .with_health_mut(dirty_id, |health| {
                health.current = health.current.saturating_sub(amount);
                if health.current == 0 {
                    health.dead = true;
                    true
                }
                else {
                    false
                }
            })
            .unwrap_or(false);
        if !killed {
            let _ = self.with_animation_mut(dirty_id, |anim| {
                anim.hit_flash = HIT_FLASH_TICKS;
            });
            self.mark_entity_dirty(dirty_id);
            return;
        }
        let _ = self.with_locomotor_mut(dirty_id, |loco| {
            loco.speed = 0;
        });
        let _ = self.with_animation_mut(dirty_id, |anim| {
            anim.hit_flash = HIT_FLASH_TICKS;
        });
        let _ = self.with_attack_mut(dirty_id, |attack| {
            attack.target = None;
        });
        let _ = self.with_movement_mut(dirty_id, |movement| {
            movement.path.clear();
            movement.destination_y = None;
            movement.destination_x = None;
            movement.move_accum = 0;
        });
        self.mark_entity_dirty(dirty_id);
        let dead_id = dirty_id;
        let attackers: Vec<_> = self
            .entities
            .iter()
            .filter_map(|o| {
                let attack = self.ecs_get::<AttackState>(o.id)?;
                (attack.target == Some(dead_id)).then_some(o.id)
            })
            .collect();
        for attacker_id in attackers {
            let _ = self.with_attack_mut(attacker_id, |attack| {
                attack.target = None;
            });
        }
        if kind == MapEntityKind::Structure {
            self.pass_grid.set_passable(x, y, true);
            self.revoke_structure_power(&house, &type_id);
        }
    }

    fn revoke_structure_power(&mut self, house: &str, type_id: &str) {
        let power = building_power(&self.definitions, type_id);
        let Some(player) = self.players.iter_mut().find(|p| p.house.as_ref() == house)
        else {
            return;
        };
        player.power_output = player.power_output.saturating_sub(power.output);
        player.power_drain = player.power_drain.saturating_sub(power.drain);
    }

    pub(crate) fn advance_turrets(&mut self) {
        let n = self.entities.len();
        for i in 0..n {
            let id = self.entities[i].id;
            if self.ecs_get::<Health>(id).map(|h| h.dead).unwrap_or(true) {
                continue;
            }
            if !self.ecs_get::<Identity>(id).map(|i| is_mobile(i.kind)).unwrap_or(false) {
                continue;
            }
            let attack_target = self.ecs_get::<AttackState>(id).and_then(|a| a.target);
            let Some(self_xf) = self.ecs_get::<Transform>(id).copied()
            else {
                continue;
            };
            let desired = if let Some(target_id) = attack_target {
                if let Some(target_xf) = self.ecs_get::<Transform>(target_id).copied() {
                    if !self.ecs_get::<Health>(target_id).map(|h| h.dead).unwrap_or(true) {
                        facing_toward(self_xf.x, self_xf.y, target_xf.x, target_xf.y)
                    }
                    else {
                        self_xf.facing
                    }
                }
                else {
                    self_xf.facing
                }
            }
            else {
                self_xf.facing
            };
            let before = self_xf.turret_facing;
            let _ = self.with_transform_mut(id, |transform| {
                turn_facing_toward(&mut transform.turret_facing, desired, TURRET_TURN_STEP);
            });
            if self.ecs_get::<Transform>(id).map(|t| t.turret_facing).unwrap_or(before) != before {
                self.mark_entity_dirty(id);
            }
        }
    }
}

fn scale_damage(base: u32, verses: &[u32; 11], armor: &str) -> u32 {
    let pct = verses[armor_index(armor)];
    ((u64::from(base) * u64::from(pct)) / 100) as u32
}
