//! 攻击、伤害、受击闪白与炮塔转向。

use ra_map::MapEntityKind;

use crate::{
    gameplay::building_power,
    spatial::{facing_toward, is_mobile, manhattan, turn_facing_toward},
    state::{
        FIRE_FLASH_TICKS, HIT_FLASH_TICKS, TURRET_TURN_STEP,
        components::{AnimationState, AttackState, CombatStats, Health, Identity, Transform},
    },
};
use ra_types::ArmorKind;

impl crate::state::BattleState {
    pub(crate) fn resolve_combat(&mut self) {
        let n = self.entities.len();
        let mut damage_events: Vec<(std::sync::Arc<str>, usize, u32)> = Vec::new();
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
            let Some(attacker_house) = self
                .ecs_get::<crate::state::components::Owner>(attacker_id)
                .map(|o| std::sync::Arc::<str>::from(crate::gameplay::house_key_of(&self.definitions, o.house)))
            else {
                continue;
            };
            let Some(attack) = self.ecs_get::<AttackState>(attacker_id)
            else {
                continue;
            };
            // 渗透 / 占领中的单位不开火。
            if attack.infiltrate_target.is_some() || attack.capture_target.is_some() {
                continue;
            }
            let attack_move = identity.mission == Some(ra_types::MissionKind::AttackMove);
            let attacker_type_key = std::sync::Arc::<str>::from(crate::gameplay::type_key_of(&self.definitions, identity.type_id));
            let Some(target_id) = attack.target.or_else(|| {
                if !attack_move {
                    return None;
                }
                let range = self.ecs_get::<CombatStats>(attacker_id).map(|s| s.attack_range).unwrap_or(0);
                self.nearest_hostile_in_range(attacker_id, range)
            })
            else {
                continue;
            };
            if attack.target.is_none() {
                let _ = self.with_attack_mut(attacker_id, |attack| {
                    attack.target = Some(target_id);
                });
            }
            let Some(ti) = self.entity_index(target_id)
            else {
                self.clear_attack_target_resume_attack_move(attacker_id, attack_move);
                continue;
            };
            let target_dead = self.ecs_get::<Health>(target_id).map(|h| h.dead).unwrap_or(true);
            if target_dead || ti == i {
                self.clear_attack_target_resume_attack_move(attacker_id, attack_move);
                continue;
            }
            // 追击：把移动目标钉在敌人当前格；攻击移动时把原目的地压入航点以便战后续行。
            let Some(target_xf) = self.ecs_get::<Transform>(target_id).copied()
            else {
                continue;
            };
            let _ = self.with_movement_mut(attacker_id, |movement| {
                let dest_changed = movement.destination_x != Some(target_xf.x) || movement.destination_y != Some(target_xf.y);
                if dest_changed {
                    if attack_move {
                        // 仅在首次改道接敌时压入最终目的地，追敌改格不再污染航点。
                        if movement.waypoints.is_empty() {
                            if let (Some(dx), Some(dy)) = (movement.destination_x, movement.destination_y) {
                                if (dx, dy) != (target_xf.x, target_xf.y) {
                                    movement.waypoints.push((dx, dy));
                                }
                            }
                        }
                    }
                    movement.destination_x = Some(target_xf.x);
                    movement.destination_y = Some(target_xf.y);
                    movement.path.clear();
                    movement.move_accum = 0;
                }
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
                let dmg = scale_damage(stats.attack_damage, &stats.attack_verses, target_stats.armor);
                let fire_report = self
                    .definitions
                    .techno
                    .get(attacker_type_key.as_ref())
                    .and_then(|t| t.primary_id)
                    .and_then(|wid| self.definitions.weapons.get_by_id(wid))
                    .map(|w| w.report.as_str())
                    .filter(|r| !r.is_empty())
                    .map(str::to_string);
                damage_events.push((attacker_house, ti, dmg));
                let cooldown_max = stats.attack_cooldown_max;
                let _ = self.with_attack_mut(attacker_id, |attack| {
                    attack.cooldown = cooldown_max;
                });
                let _ = self.with_animation_mut(attacker_id, |anim| {
                    anim.fire_flash = FIRE_FLASH_TICKS;
                });
                self.mark_entity_dirty(attacker_id);
                if let Some(report) = fire_report {
                    self.push_battle_sfx_cue(report);
                }
            }
        }
        for (killer_house, ti, dmg) in damage_events {
            self.apply_damage_credited(ti, dmg, Some(killer_house.as_ref()));
        }
    }

    /// 清除攻击目标；若为攻击移动则立刻弹出航点续行最终目的地。
    fn clear_attack_target_resume_attack_move(&mut self, attacker_id: ra_types::EntityId, attack_move: bool) {
        let _ = self.with_attack_mut(attacker_id, |attack| {
            attack.target = None;
        });
        if !attack_move {
            return;
        }
        let resume = self.with_movement_mut(attacker_id, |movement| {
            if let Some((nx, ny)) = movement.waypoints.first().copied() {
                movement.waypoints.remove(0);
                movement.destination_x = Some(nx);
                movement.destination_y = Some(ny);
                movement.path.clear();
                movement.move_accum = 0;
                true
            }
            else {
                false
            }
        });
        if resume == Some(true) {
            if let Some(idx) = self.entity_index(attacker_id) {
                self.repath_entity_at(idx);
            }
        }
    }

    /// 射程内最近的异阵营存活目标（机动单位或建筑）。
    fn nearest_hostile_in_range(&self, from: ra_types::EntityId, range: u32) -> Option<ra_types::EntityId> {
        if range == 0 {
            return None;
        }
        if self.ecs_get::<Health>(from).map(|h| h.dead).unwrap_or(true) {
            return None;
        }
        let owner = crate::gameplay::house_key_of(&self.definitions, self.ecs_get::<crate::state::components::Owner>(from)?.house).to_string();
        let xf = self.ecs_get::<Transform>(from).copied()?;
        self.entities
            .iter()
            .filter_map(|e| {
                let id = e.id;
                if id == from {
                    return None;
                }
                if self.ecs_get::<Health>(id).map(|h| h.dead).unwrap_or(true) {
                    return None;
                }
                let identity = self.ecs_get::<Identity>(id)?;
                if !matches!(identity.kind, MapEntityKind::Unit | MapEntityKind::Infantry | MapEntityKind::Aircraft | MapEntityKind::Structure)
                {
                    return None;
                }
                let other = self.ecs_get::<crate::state::components::Owner>(id)?;
                if crate::gameplay::house_key_of(&self.definitions, other.house) == owner {
                    return None;
                }
                let ox = self.ecs_get::<Transform>(id)?;
                let dist = manhattan(xf.x, xf.y, ox.x, ox.y);
                (dist <= range).then_some((dist, id))
            })
            .min_by_key(|(dist, _)| *dist)
            .map(|(_, id)| id)
    }

    pub(crate) fn tick_hit_flash(&mut self) {
        let ids: Vec<_> = self
            .entities
            .iter()
            .filter_map(|e| {
                let anim = self.ecs_get::<AnimationState>(e.id)?;
                (anim.hit_flash > 0 || anim.fire_flash > 0).then_some(e.id)
            })
            .collect();
        for id in ids {
            let _ = self.with_animation_mut(id, |anim| {
                if anim.hit_flash > 0 {
                    anim.hit_flash -= 1;
                }
                if anim.fire_flash > 0 {
                    anim.fire_flash -= 1;
                }
            });
            self.mark_entity_dirty(id);
        }
    }

    pub(crate) fn apply_damage(&mut self, index: usize, amount: u32) {
        self.apply_damage_credited(index, amount, None);
    }

    /// 造成伤害；若击杀且 `killer_house` 与受害者不同阵营，则给击杀方记 `kills`。
    pub(crate) fn apply_damage_credited(&mut self, index: usize, amount: u32, killer_house: Option<&str>) {
        if index >= self.entities.len() || amount == 0 {
            return;
        }
        let dirty_id = self.entities[index].id;
        if self.ecs_get::<Health>(dirty_id).map(|h| h.dead).unwrap_or(true) {
            return;
        }
        let house = self
            .ecs_get::<crate::state::components::Owner>(dirty_id)
            .map(|o| std::sync::Arc::<str>::from(crate::gameplay::house_key_of(&self.definitions, o.house)));
        let kind = self.ecs_get::<Identity>(dirty_id).map(|i| i.kind);
        let type_id = self.ecs_get::<Identity>(dirty_id).map(|i| i.type_id);
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
        // 建筑受击：引擎排队遇袭 EVA（近距/时间窗去重）。壳层不再用 `hit_flash` 边沿。
        if kind == MapEntityKind::Structure {
            self.try_announce_base_under_attack(house.as_ref(), x, y);
        }
        if !killed {
            let _ = self.with_animation_mut(dirty_id, |anim| {
                anim.hit_flash = HIT_FLASH_TICKS;
            });
            self.mark_entity_dirty(dirty_id);
            return;
        }
        if let Some(killer) = killer_house {
            if killer != house.as_ref() {
                if let Some(player) = self.players.iter_mut().find(|p| p.house.eq_ignore_ascii_case(killer)) {
                    player.kills = player.kills.saturating_add(1);
                }
            }
        }
        // 阵亡播报只挂真实击杀。`Deploy` 等同 id 变形不走本路径，故不会误触。
        if is_mobile(kind) {
            self.push_eva_cue(house.as_ref(), "EVA_UnitLost");
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
            movement.waypoints.clear();
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
            let attack_move = self
                .ecs_get::<Identity>(attacker_id)
                .map(|identity| identity.mission == Some(ra_types::MissionKind::AttackMove))
                .unwrap_or(false);
            self.clear_attack_target_resume_attack_move(attacker_id, attack_move);
        }
        if kind == MapEntityKind::Structure {
            let foundation = self.definitions.structures.get_by_id(type_id).map(|s| s.foundation.clone()).unwrap_or_default();
            let type_key = crate::gameplay::type_key_of(&self.definitions, type_id).to_string();
            self.unseal_structure_footprint(x, y, foundation.width, foundation.height);
            self.revoke_structure_power(&house, type_key.as_str());
        }
    }

    pub(crate) fn revoke_structure_power(&mut self, house: &str, type_id: &str) {
        let power = building_power(&self.definitions, type_id);
        let Some(player) = self.players.iter_mut().find(|p| p.house.eq_ignore_ascii_case(house))
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

fn scale_damage(base: u32, verses: &[u32; 11], armor: ArmorKind) -> u32 {
    let pct = verses[armor.index()];
    ((u64::from(base) * u64::from(pct)) / 100) as u32
}
