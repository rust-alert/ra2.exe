//! 工厂生产队列、出厂与集结。

use ra_assets::TechnoKind;
use ra_map::MapEntityKind;

use crate::navigation::repath_at;
use crate::rules::{factory_matches_unit, verses_for};
use crate::{ATTACK_COOLDOWN_TICKS, World, WorldEntity};

impl World {
    pub(crate) fn advance_production(&mut self) {
        let mut spawns: Vec<(usize, String)> = Vec::new();
        for (index, e) in self.entities.iter_mut().enumerate() {
            if e.dead {
                continue;
            }
            let Some((type_id, remaining)) = e.produce_queue.as_mut()
            else {
                continue;
            };
            if *remaining > 1 {
                *remaining -= 1;
                continue;
            }
            let type_id = type_id.clone();
            e.produce_queue = None;
            spawns.push((index, type_id));
        }
        for (factory_index, type_id) in spawns {
            self.spawn_produced_unit(factory_index, &type_id);
        }
    }

    pub(crate) fn spawn_produced_unit(&mut self, factory_index: usize, type_id: &str) {
        let Some(tt) = self.techno_types.get(type_id).cloned()
        else {
            return;
        };
        let factory = &self.entities[factory_index];
        let owner = factory.owner.clone();
        let fx = factory.x;
        let fy = factory.y;
        let rally = match (factory.rally_x, factory.rally_y) {
            (Some(rx), Some(ry)) => Some((rx, ry)),
            _ => None,
        };
        let Some((x, y)) = self.find_spawn_cell(fx, fy)
        else {
            return;
        };
        let kind = match tt.kind {
            TechnoKind::Infantry => MapEntityKind::Infantry,
            TechnoKind::Vehicle => MapEntityKind::Unit,
            TechnoKind::Aircraft => MapEntityKind::Aircraft,
            TechnoKind::Building => return,
        };
        let max_health = tt.strength.max(1);
        let id = self.alloc_entity_id();
        let unit_index = self.entities.len();
        self.entities.push(WorldEntity {
            id,
            kind,
            owner,
            type_id: type_id.to_ascii_uppercase(),
            x,
            y,
            facing: 0,
            turret_facing: 0,
            sub_cell: 0,
            health: max_health,
            max_health,
            speed: tt.speed,
            armor: tt.armor.clone(),
            attack_range: if tt.range > 0 { tt.range } else { tt.sight.max(1) },
            attack_damage: if tt.damage > 0 { tt.damage } else { (tt.strength / 4).max(1) },
            attack_cooldown_max: if tt.rof > 0 { tt.rof } else { ATTACK_COOLDOWN_TICKS },
            attack_verses: verses_for(&self.warheads, &tt.warhead),
            techno_kind: Some(tt.kind),
            target_x: None,
            target_y: None,
            path: Vec::new(),
            move_accum: 0,
            hva_frame: 0,
            attack_target: None,
            attack_cooldown: 0,
            ore_trip_accum: 0,
            produce_queue: None,
            rally_x: None,
            rally_y: None,
            hit_flash: 0,
            dead: false,
        });
        if let Some((rx, ry)) = rally {
            let e = &mut self.entities[unit_index];
            e.target_x = Some(rx);
            e.target_y = Some(ry);
            e.path.clear();
            e.move_accum = 0;
            repath_at(&mut self.entities, unit_index, &self.pass_grid);
        }
    }

    pub(crate) fn find_spawn_cell(&self, fx: u16, fy: u16) -> Option<(u16, u16)> {
        const DELTAS: [(i32, i32); 8] = [(1, 0), (0, 1), (-1, 0), (0, -1), (1, 1), (-1, 1), (-1, -1), (1, -1)];
        for (dx, dy) in DELTAS {
            let x = i32::from(fx) + dx;
            let y = i32::from(fy) + dy;
            if x < 0 || y < 0 {
                continue;
            }
            let (x, y) = (x as u16, y as u16);
            if self.can_place_structure(x, y) {
                return Some((x, y));
            }
        }
        None
    }

    pub(crate) fn find_factory(&self, house: &str, kind: TechnoKind) -> Option<usize> {
        self.entities.iter().position(|e| {
            !e.dead && e.owner == house && e.kind == MapEntityKind::Structure && factory_matches_unit(&e.type_id, kind)
        })
    }

    pub(crate) fn find_idle_factory(&self, house: &str, kind: TechnoKind) -> Option<usize> {
        self.entities.iter().position(|e| {
            !e.dead
                && e.owner == house
                && e.kind == MapEntityKind::Structure
                && e.produce_queue.is_none()
                && factory_matches_unit(&e.type_id, kind)
        })
    }
}
