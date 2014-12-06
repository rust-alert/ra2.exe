//! 攻击、伤害、受击闪白与炮塔转向。

use ra_map::MapEntityKind;

use crate::{
    gameplay::building_power,
    spatial::{facing_toward, is_mobile, manhattan, turn_facing_toward},
    state::{HIT_FLASH_TICKS, TURRET_TURN_STEP},
};
use ra_assets::armor_index;

impl crate::state::MatchState {
    pub(crate) fn resolve_combat(&mut self) {
        let n = self.entities.len();
        let mut damage_events: Vec<(usize, u32)> = Vec::new();
        for i in 0..n {
            if self.entities[i].dead || !is_mobile(self.entities[i].kind) {
                continue;
            }
            let attacker_id = self.entities[i].id;
            let Some(target_id) = self.entities[i].attack_target
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
            if self.entities[ti].dead || ti == i {
                let _ = self.with_attack_mut(attacker_id, |attack| {
                    attack.target = None;
                });
                continue;
            }
            // 追击：把移动目标钉在敌人当前格。
            let (tx, ty) = (self.entities[ti].x, self.entities[ti].y);
            let _ = self.with_movement_mut(attacker_id, |movement| {
                movement.destination_x = Some(tx);
                movement.destination_y = Some(ty);
            });

            if self.entities[i].attack_cooldown > 0 {
                let _ = self.with_attack_mut(attacker_id, |attack| {
                    attack.cooldown = attack.cooldown.saturating_sub(1);
                });
                continue;
            }
            let dist = manhattan(self.entities[i].x, self.entities[i].y, self.entities[ti].x, self.entities[ti].y);
            if dist <= self.entities[i].attack_range {
                let base = self.entities[i].attack_damage;
                let verses = self.entities[i].attack_verses;
                let armor = self.entities[ti].armor.as_str();
                let dmg = scale_damage(base, &verses, armor);
                damage_events.push((ti, dmg));
                let cooldown_max = self.entities[i].attack_cooldown_max;
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
            .filter(|e| e.hit_flash > 0)
            .map(|e| e.id)
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
        if index >= self.entities.len() || self.entities[index].dead || amount == 0 {
            return;
        }
        let house = self.entities[index].owner.clone();
        let kind = self.entities[index].kind;
        let type_id = self.entities[index].type_id.clone();
        let (x, y) = (self.entities[index].x, self.entities[index].y);
        let dirty_id = self.entities[index].id;
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
        {
            let e = &mut self.entities[index];
            if !killed {
                let _ = self.with_animation_mut(dirty_id, |anim| {
                    anim.hit_flash = HIT_FLASH_TICKS;
                });
                self.mark_entity_dirty(dirty_id);
                return;
            }
            e.speed = 0;
        }
        let _ = self.with_animation_mut(dirty_id, |anim| {
            anim.hit_flash = HIT_FLASH_TICKS;
        });
        let _ = self.with_attack_mut(dirty_id, |attack| {
            attack.target = None;
        });
        let _ = self.with_movement_mut(dirty_id, |movement| {
            movement.path.clear();
            movement.destination_x = None;
            movement.destination_y = None;
            movement.move_accum = 0;
        });
        self.mark_entity_dirty(dirty_id);
        let dead_id = dirty_id;
        let attackers: Vec<_> = self
            .entities
            .iter()
            .filter(|o| o.attack_target == Some(dead_id))
            .map(|o| o.id)
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
            if self.entities[i].dead || !is_mobile(self.entities[i].kind) {
                continue;
            }
            let desired = if let Some(target_id) = self.entities[i].attack_target {
                if let Some(ti) = self.entity_index(target_id) {
                    if !self.entities[ti].dead {
                        facing_toward(self.entities[i].x, self.entities[i].y, self.entities[ti].x, self.entities[ti].y)
                    }
                    else {
                        self.entities[i].facing
                    }
                }
                else {
                    self.entities[i].facing
                }
            }
            else {
                self.entities[i].facing
            };
            let before = self.entities[i].turret_facing;
            let id = self.entities[i].id;
            let _ = self.with_transform_mut(id, |transform| {
                turn_facing_toward(&mut transform.turret_facing, desired, TURRET_TURN_STEP);
            });
            if self.entities[i].turret_facing != before {
                self.mark_entity_dirty(id);
            }
        }
    }
}

fn scale_damage(base: u32, verses: &[u32; 11], armor: &str) -> u32 {
    let pct = verses[armor_index(armor)];
    ((u64::from(base) * u64::from(pct)) / 100) as u32
}
