//! 攻击、伤害、受击闪白与炮塔转向。

use ra_map::MapEntityKind;

use crate::{
    gameplay::building_power_delta,
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
            let Some(ti) = self.entities[i].attack_target
            else {
                continue;
            };
            if ti >= n || self.entities[ti].dead || ti == i {
                self.entities[i].attack_target = None;
                continue;
            }
            // 追击：把移动目标钉在敌人当前格。
            self.entities[i].target_x = Some(self.entities[ti].x);
            self.entities[i].target_y = Some(self.entities[ti].y);

            if self.entities[i].attack_cooldown > 0 {
                self.entities[i].attack_cooldown -= 1;
                continue;
            }
            let dist = manhattan(self.entities[i].x, self.entities[i].y, self.entities[ti].x, self.entities[ti].y);
            if dist <= self.entities[i].attack_range {
                let base = self.entities[i].attack_damage;
                let verses = self.entities[i].attack_verses;
                let armor = self.entities[ti].armor.as_str();
                let dmg = scale_damage(base, &verses, armor);
                damage_events.push((ti, dmg));
                self.entities[i].attack_cooldown = self.entities[i].attack_cooldown_max;
            }
        }
        for (ti, dmg) in damage_events {
            self.apply_damage(ti, dmg);
        }
    }

    pub(crate) fn tick_hit_flash(&mut self) {
        for e in &mut self.entities {
            if e.hit_flash > 0 {
                e.hit_flash -= 1;
            }
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
        {
            let e = &mut self.entities[index];
            e.health = e.health.saturating_sub(amount);
            e.hit_flash = HIT_FLASH_TICKS;
            if e.health > 0 {
                return;
            }
            e.dead = true;
            e.speed = 0;
            e.path.clear();
            e.target_x = None;
            e.target_y = None;
            e.attack_target = None;
            e.move_accum = 0;
        }
        let dead_i = index;
        for o in self.entities.iter_mut() {
            if o.attack_target == Some(dead_i) {
                o.attack_target = None;
            }
        }
        if kind == MapEntityKind::Structure {
            self.pass_grid.set_passable(x, y, true);
            self.revoke_structure_power(&house, &type_id);
        }
    }

    fn revoke_structure_power(&mut self, house: &str, type_id: &str) {
        let Some(player) = self.players.iter_mut().find(|p| p.house == house)
        else {
            return;
        };
        let power = building_power_delta(type_id);
        if power >= 0 {
            player.power_output = player.power_output.saturating_sub(power);
        }
        else {
            player.power_drain = player.power_drain.saturating_sub(-power);
        }
    }

    pub(crate) fn advance_turrets(&mut self) {
        let n = self.entities.len();
        for i in 0..n {
            if self.entities[i].dead || !is_mobile(self.entities[i].kind) {
                continue;
            }
            let desired = if let Some(ti) = self.entities[i].attack_target {
                if ti < n && !self.entities[ti].dead {
                    facing_toward(self.entities[i].x, self.entities[i].y, self.entities[ti].x, self.entities[ti].y)
                }
                else {
                    self.entities[i].facing
                }
            }
            else {
                self.entities[i].facing
            };
            turn_facing_toward(&mut self.entities[i].turret_facing, desired, TURRET_TURN_STEP);
        }
    }
}

fn scale_damage(base: u32, verses: &[u32; 11], armor: &str) -> u32 {
    let pct = verses[armor_index(armor)];
    ((u64::from(base) * u64::from(pct)) / 100) as u32
}
