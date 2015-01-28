//! 工厂生产队列、出厂与集结。

use ra_assets::TechnoKind;
use ra_map::MapEntityKind;
use std::sync::Arc;

use crate::{
    gameplay::{factory_matches_unit, verses_for},
    state::{
        ATTACK_COOLDOWN_TICKS,
        components::{
            AnimationState, AttackState, CombatStats, EntitySpawnBundle, HarvesterState, Health, Identity, Locomotor,
            MovementState, Owner, ProductionQueue, Transform,
        },
    },
};

impl crate::state::BattleState {
    pub(crate) fn advance_production(&mut self) {
        let mut spawns: Vec<(usize, Arc<str>)> = Vec::new();
        let n = self.entities.len();
        for index in 0..n {
            let id = self.entities[index].id;
            if self.ecs_get::<Health>(id).map(|h| h.dead).unwrap_or(true) {
                continue;
            }
            let finished = self
                .with_production_mut(id, |queue| {
                    let Some((type_id, remaining)) = queue.item.as_mut()
                    else {
                        return None;
                    };
                    if *remaining > 1 {
                        *remaining -= 1;
                        return None;
                    }
                    let type_id = type_id.clone();
                    queue.item = None;
                    Some(type_id)
                })
                .flatten();
            if let Some(type_id) = finished {
                spawns.push((index, type_id));
            }
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
        let factory_id = self.entities[factory_index].id;
        let Some(owner) = self.ecs_get::<Owner>(factory_id).map(|o| o.house.clone())
        else {
            return;
        };
        let Some(factory_xf) = self.ecs_get::<Transform>(factory_id).copied()
        else {
            return;
        };
        let rally = self.ecs_get::<ProductionQueue>(factory_id).and_then(|q| match (q.rally_x, q.rally_y) {
            (Some(rx), Some(ry)) => Some((rx, ry)),
            _ => None,
        });
        let Some((x, y)) = self.find_spawn_cell(factory_xf.x, factory_xf.y)
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
        let unit_index = self.spawn_from_bundle(EntitySpawnBundle {
            identity: Identity {
                entity_id: id,
                type_id: Arc::<str>::from(type_id.to_ascii_uppercase()),
                kind,
            },
            owner: Owner { house: owner },
            transform: Transform { x, y, facing: 0, turret_facing: 0, sub_cell: 0 },
            health: Health { current: max_health, maximum: max_health, dead: false },
            locomotor: Locomotor { speed: tt.speed },
            movement: MovementState {
                destination_x: None,
                destination_y: None,
                path: Vec::new(),
                move_accum: 0,
            },
            combat: CombatStats {
                armor: tt.armor.clone(),
                attack_range: if tt.range > 0 { tt.range } else { tt.sight.max(1) },
                attack_damage: if tt.damage > 0 { tt.damage } else { (tt.strength / 4).max(1) },
                attack_cooldown_max: if tt.rof > 0 { tt.rof } else { ATTACK_COOLDOWN_TICKS },
                attack_verses: verses_for(&self.definitions, &tt.warhead),
                techno_kind: Some(techno_kind),
            },
            attack: AttackState { target: None, cooldown: 0 },
            production: ProductionQueue { item: None, rally_x: None, rally_y: None },
            harvester: HarvesterState { ore_trip_accum: 0 },
            animation: AnimationState { hva_frame: 0, hit_flash: 0 },
        });
        self.mark_entity_dirty(id);
        if let Some((rx, ry)) = rally {
            let _ = self.with_movement_mut(id, |movement| {
                movement.destination_x = Some(rx);
                movement.destination_y = Some(ry);
                movement.path.clear();
                movement.move_accum = 0;
            });
            self.repath_entity_at(unit_index);
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
            let id = e.id;
            !self.ecs_get::<Health>(id).map(|h| h.dead).unwrap_or(true)
                && self.ecs_get::<Owner>(id).map(|o| o.house.as_ref() == house).unwrap_or(false)
                && self.ecs_get::<Identity>(id).map(|i| i.kind == MapEntityKind::Structure).unwrap_or(false)
                && self
                    .ecs_get::<Identity>(id)
                    .map(|i| factory_matches_unit(&self.definitions, &i.type_id, class))
                    .unwrap_or(false)
        })
    }

    pub(crate) fn find_idle_factory(&self, house: &str, kind: TechnoKind) -> Option<usize> {
        let class = techno_kind_to_class(kind);
        self.entities.iter().position(|e| {
            let id = e.id;
            !self.ecs_get::<Health>(id).map(|h| h.dead).unwrap_or(true)
                && self.ecs_get::<Owner>(id).map(|o| o.house.as_ref() == house).unwrap_or(false)
                && self.ecs_get::<Identity>(id).map(|i| i.kind == MapEntityKind::Structure).unwrap_or(false)
                && self.ecs_get::<ProductionQueue>(id).map(|q| q.item.is_none()).unwrap_or(false)
                && self
                    .ecs_get::<Identity>(id)
                    .map(|i| factory_matches_unit(&self.definitions, &i.type_id, class))
                    .unwrap_or(false)
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
