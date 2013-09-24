//! 确定性状态摘要（锁步校验用）。

use crate::{game::GameCommand, state::MatchState};

impl MatchState {
    pub(crate) fn rehash(&mut self) {
        let mut h = self.tick;
        h = h.wrapping_mul(1099511628211).wrapping_add(self.edition.as_str().len() as u64);
        h = h.wrapping_mul(1099511628211).wrapping_add(self.entities.len() as u64);
        h = h
            .wrapping_mul(1099511628211)
            .wrapping_add(self.last_input_frame.tick)
            .wrapping_add(self.last_input_frame.commands.len() as u64);
        for cmd in &self.last_input_frame.commands {
            h = hash_command(h, cmd);
        }
        for e in &self.entities {
            h = h
                .wrapping_mul(1099511628211)
                .wrapping_add(e.id.0)
                .wrapping_add(e.x as u64)
                .wrapping_add((e.y as u64) << 16)
                .wrapping_add((e.facing as u64) << 32)
                .wrapping_add((e.turret_facing as u64) << 40)
                .wrapping_add(u64::from(e.health))
                .wrapping_add(u64::from(e.max_health).wrapping_shl(1))
                .wrapping_add(u64::from(e.speed).wrapping_shl(2))
                .wrapping_add(u64::from(e.attack_range).wrapping_shl(3))
                .wrapping_add(u64::from(e.attack_damage).wrapping_shl(4))
                .wrapping_add(u64::from(e.attack_cooldown_max).wrapping_shl(5))
                .wrapping_add(u64::from(e.dead))
                .wrapping_add(u64::from(e.hva_frame) << 8)
                .wrapping_add(u64::from(e.attack_cooldown) << 24)
                .wrapping_add(u64::from(e.ore_trip_accum) << 8)
                .wrapping_add(u64::from(e.hit_flash) << 16)
                .wrapping_add(e.attack_target.map(|i| i as u64 + 1).unwrap_or(0) << 32);
            for b in e.armor.as_bytes() {
                h = h.wrapping_mul(1099511628211).wrapping_add(u64::from(*b));
            }
            for v in e.attack_verses {
                h = h.wrapping_mul(1099511628211).wrapping_add(u64::from(v));
            }
            if let Some((ref qid, rem)) = e.produce_queue {
                h = h.wrapping_mul(1099511628211).wrapping_add(u64::from(rem));
                for b in qid.as_bytes() {
                    h = h.wrapping_mul(1099511628211).wrapping_add(u64::from(*b));
                }
            }
            h = h
                .wrapping_mul(1099511628211)
                .wrapping_add(e.rally_x.map(u64::from).unwrap_or(0))
                .wrapping_add(e.rally_y.map(|v| u64::from(v) << 16).unwrap_or(0));
            for b in e.type_id.as_bytes() {
                h = h.wrapping_mul(1099511628211).wrapping_add(u64::from(*b));
            }
            for b in e.owner.as_bytes() {
                h = h.wrapping_mul(1099511628211).wrapping_add(u64::from(*b));
            }
        }
        for p in &self.players {
            h = h
                .wrapping_mul(1099511628211)
                .wrapping_add(u64::from(p.id.0))
                .wrapping_add(p.funds as u64)
                .wrapping_add(p.funds_spent as u64)
                .wrapping_add((p.power_output as u64) << 16)
                .wrapping_add((p.power_drain as u64) << 32);
            for b in p.house.as_bytes() {
                h = h.wrapping_mul(1099511628211).wrapping_add(u64::from(*b));
            }
        }
        self.state_hash = h;
    }
}

pub(crate) fn hash_command(mut h: u64, cmd: &GameCommand) -> u64 {
    match *cmd {
        GameCommand::MoveTo { entity_index, x, y } => {
            h = h.wrapping_mul(1099511628211).wrapping_add(1);
            h = h
                .wrapping_mul(1099511628211)
                .wrapping_add(entity_index as u64)
                .wrapping_add((x as u64) << 16)
                .wrapping_add((y as u64) << 32);
        }
        GameCommand::Attack { attacker_index, target_index } => {
            h = h.wrapping_mul(1099511628211).wrapping_add(2);
            h = h.wrapping_mul(1099511628211).wrapping_add(attacker_index as u64).wrapping_add((target_index as u64) << 16);
        }
        GameCommand::Deploy { entity_index } => {
            h = h.wrapping_mul(1099511628211).wrapping_add(3);
            h = h.wrapping_mul(1099511628211).wrapping_add(entity_index as u64);
        }
        GameCommand::PlaceBuilding { player, ref type_id, x, y } => {
            h = h.wrapping_mul(1099511628211).wrapping_add(4);
            h = h
                .wrapping_mul(1099511628211)
                .wrapping_add(u64::from(player.0))
                .wrapping_add((x as u64) << 8)
                .wrapping_add((y as u64) << 24);
            for b in type_id.as_bytes() {
                h = h.wrapping_mul(1099511628211).wrapping_add(u64::from(*b));
            }
        }
        GameCommand::Produce { player, ref type_id } => {
            h = h.wrapping_mul(1099511628211).wrapping_add(5);
            h = h.wrapping_mul(1099511628211).wrapping_add(u64::from(player.0));
            for b in type_id.as_bytes() {
                h = h.wrapping_mul(1099511628211).wrapping_add(u64::from(*b));
            }
        }
        GameCommand::SetRallyPoint { factory_index, x, y } => {
            h = h.wrapping_mul(1099511628211).wrapping_add(6);
            h = h
                .wrapping_mul(1099511628211)
                .wrapping_add(factory_index as u64)
                .wrapping_add((x as u64) << 16)
                .wrapping_add((y as u64) << 32);
        }
    }
    h
}
