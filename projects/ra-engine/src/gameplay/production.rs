//! 工厂生产队列、出厂与集结。

use std::sync::Arc;
use ra_assets::TechnoKind;
use ra_map::MapEntityKind;

use crate::{
    gameplay::{factory_matches_unit, verses_for},
    spatial::repath_at,
    state::{ATTACK_COOLDOWN_TICKS, WorldEntity},
};

impl crate::state::MatchState {
    pub(crate) fn advance_production(&mut self) {
        let mut spawns: Vec<(usize, Arc<str>)> = Vec::new();
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
            self.spawn_produced_unit(factory_index, type_id.as_ref());
        }
    }

    pub(crate) fn spawn_produced_unit(&mut self, factory_index: usize, type_id: &str) {
        let Some(tt) = self.definitions.techno.get(type_id).cloned()
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
        let kind = match tt.class {
            ra_types::TechnoClass::Infantry => MapEntityKind::Infantry,
            ra_types::TechnoClass::Vehicle => MapEntityKind::Unit,
            ra_types::TechnoClass::Aircraft => MapEntityKind::Aircraft,
            ra_types::TechnoClass::Building => return,
        };
        let techno_kind = match tt.class {
            ra_types::TechnoClass::Infantry => TechnoKind::Infantry,
            ra_types::TechnoClass::Vehicle => TechnoKind::Vehicle,
            ra_types::TechnoClass::Aircraft => TechnoKind::Aircraft,
            ra_types::TechnoClass::Building => return,
        };
        let max_health = tt.strength.max(1);
        let id = self.alloc_entity_id();
        let unit_index = self.entities.len();
        self.entities.push(WorldEntity {
            id,
            kind,
            owner,
            type_id: Arc::<str>::from(type_id.to_ascii_uppercase()),
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
            attack_verses: verses_for(&self.definitions, &tt.warhead),
            techno_kind: Some(techno_kind),
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
        self.mark_entity_dirty(id);
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
        let class = techno_kind_to_class(kind);
        self.entities.iter().position(|e| {
            !e.dead
                && e.owner.as_ref() == house
                && e.kind == MapEntityKind::Structure
                && factory_matches_unit(&self.definitions, &e.type_id, class)
        })
    }

    pub(crate) fn find_idle_factory(&self, house: &str, kind: TechnoKind) -> Option<usize> {
        let class = techno_kind_to_class(kind);
        self.entities.iter().position(|e| {
            !e.dead
                && e.owner.as_ref() == house
                && e.kind == MapEntityKind::Structure
                && e.produce_queue.is_none()
                && factory_matches_unit(&self.definitions, &e.type_id, class)
        })
    }
}

fn techno_kind_to_class(kind: TechnoKind) -> ra_types::TechnoClass {
    match kind {
        TechnoKind::Infantry => ra_types::TechnoClass::Infantry,
        TechnoKind::Vehicle => ra_types::TechnoClass::Vehicle,
        TechnoKind::Aircraft => ra_types::TechnoClass::Aircraft,
        TechnoKind::Building => ra_types::TechnoClass::Building,
    }
}
