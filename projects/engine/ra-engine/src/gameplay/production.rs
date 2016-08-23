//! 工厂生产队列、出厂与集结。

use std::sync::Arc;

use ra_map::MapEntityKind;
use ra_types::{TechnoClass, TechnoDefinition};

use crate::{
    gameplay::{factory_matches_unit, verses_for},
    state::{
        ATTACK_COOLDOWN_TICKS, BUILD_TIME_TICKS_PER_UNIT, PRODUCE_TICKS,
        components::{
            AnimationState, AttackState, CombatStats, EntitySpawnBundle, HarvesterState, Health, Identity, Locomotor, MovementState, Owner,
            ProductionQueue, Transform,
        },
    },
};

/// 将冻结定义中的 `BuildTime` 转为生产队列剩余 tick。
///
/// `build_time == 0`（缺省）回退 [`PRODUCE_TICKS`]。否则为 `build_time * BUILD_TIME_TICKS_PER_UNIT`，至少 1。
pub fn produce_ticks_for(techno: &TechnoDefinition) -> u32 {
    if techno.build_time == 0 { PRODUCE_TICKS } else { techno.build_time.saturating_mul(BUILD_TIME_TICKS_PER_UNIT).max(1) }
}

impl crate::state::BattleState {
    #[doc(hidden)]
    pub fn advance_production(&mut self) {
        let mut unit_spawns: Vec<(usize, Arc<str>)> = Vec::new();
        let mut building_ready: Vec<(usize, Arc<str>)> = Vec::new();
        let n = self.entities.len();
        for index in 0..n {
            let id = self.entities[index].id;
            if self.ecs_get::<Health>(id).map(|h| h.dead).unwrap_or(true) {
                continue;
            }
            let tick = self.tick;
            let low_power = self
                .ecs_get::<Owner>(id)
                .and_then(|o| self.players.iter().find(|p| p.house.as_ref() == o.house.as_ref()))
                .is_some_and(|p| p.low_power());
            let finished = self
                .with_production_mut(id, |queue| {
                    let Some((type_id, remaining)) = queue.item.as_mut()
                    else {
                        return None;
                    };
                    // 低电：每隔一 tick 才推进，等效半速（与供电不足反馈一致）。
                    if low_power && tick % 2 == 1 {
                        return None;
                    }
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
                let is_building = self.definitions.techno.get(type_id.as_ref()).is_some_and(|t| t.class == TechnoClass::Building);
                if is_building {
                    building_ready.push((index, type_id));
                }
                else {
                    unit_spawns.push((index, type_id));
                }
            }
        }
        for (factory_index, type_id) in building_ready {
            let factory_id = self.entities[factory_index].id;
            let _ = self.with_production_mut(factory_id, |queue| {
                queue.ready = Some(type_id.clone());
            });
            self.mark_entity_dirty(factory_id);
            if let Some(owner) = self.ecs_get::<Owner>(factory_id).map(|o| o.house.clone()) {
                self.push_eva_cue(owner.as_ref(), "EVA_ConstructionComplete");
            }
        }
        for (factory_index, type_id) in unit_spawns {
            self.spawn_produced_unit(factory_index, type_id.as_ref());
        }
    }

    #[doc(hidden)]
    pub fn spawn_produced_unit(&mut self, factory_index: usize, type_id: &str) {
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
            TechnoClass::Infantry => MapEntityKind::Infantry,
            TechnoClass::Vehicle => MapEntityKind::Unit,
            TechnoClass::Aircraft => MapEntityKind::Aircraft,
            TechnoClass::Building => return,
        };
        let techno_class = tt.class;
        let promoted = self.players.iter().find(|p| p.house.as_ref() == owner.as_ref()).is_some_and(|p| match tt.class {
            TechnoClass::Infantry => p.promoted_infantry,
            TechnoClass::Vehicle => p.promoted_vehicle,
            _ => false,
        });
        let base_health = tt.strength.max(1);
        let max_health =
            if promoted { base_health.saturating_mul(5).saturating_div(4).max(base_health.saturating_add(1)) } else { base_health };
        let weapon = self.definitions.weapons.get_by_id(tt.primary_id);
        let attack_range = weapon
            .map(|w| if w.range > 0 { w.range } else { tt.sight.max(1) })
            .unwrap_or_else(|| if tt.range > 0 { tt.range } else { tt.sight.max(1) });
        // 无 Primary / Damage=0 保持 0，禁止用 Strength 发明伤害。
        let attack_damage = weapon.map(|w| w.damage).unwrap_or(tt.damage);
        let attack_cooldown_max = weapon
            .map(|w| if w.rof > 0 { w.rof } else { ATTACK_COOLDOWN_TICKS })
            .unwrap_or_else(|| if tt.rof > 0 { tt.rof } else { ATTACK_COOLDOWN_TICKS });
        let warhead_id = weapon.map(|w| w.warhead_id).unwrap_or(tt.warhead_id);
        let attack_verses = verses_for(&self.definitions, warhead_id);
        let id = self.alloc_entity_id();
        let unit_index = self.spawn_from_bundle(EntitySpawnBundle {
            identity: Identity {
                entity_id: id,
                type_id: Arc::<str>::from(type_id.to_ascii_uppercase()),
                kind,
                mission: String::new(),
                tag: String::new(),
            },
            owner: Owner { house: owner.clone() },
            transform: Transform { x, y, facing: 0, turret_facing: 0, sub_cell: 0 },
            health: Health { current: max_health, maximum: max_health, dead: false },
            locomotor: Locomotor { speed: tt.speed },
            movement: MovementState { destination_x: None, destination_y: None, waypoints: Vec::new(), path: Vec::new(), move_accum: 0 },
            combat: CombatStats {
                armor: tt.armor.clone(),
                attack_range,
                attack_damage,
                attack_cooldown_max,
                attack_verses,
                techno_class: Some(techno_class),
            },
            attack: AttackState { target: None, cooldown: 0, infiltrate_target: None, capture_target: None },
            production: ProductionQueue { item: None, ready: None, rally_x: None, rally_y: None },
            harvester: HarvesterState { ore_trip_accum: 0, cargo: 0 },
            animation: AnimationState { hva_frame: 0, hit_flash: 0 },
        });
        self.mark_entity_dirty(id);
        if let Some(player) = self.players.iter_mut().find(|p| p.house.as_ref() == owner.as_ref()) {
            player.built = player.built.saturating_add(1);
        }
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

    #[doc(hidden)]
    pub fn find_spawn_cell(&self, fx: u16, fy: u16) -> Option<(u16, u16)> {
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

    #[doc(hidden)]
    pub fn find_factory(&self, house: &str, kind: TechnoClass) -> Option<usize> {
        self.entities.iter().position(|e| {
            let id = e.id;
            !self.ecs_get::<Health>(id).map(|h| h.dead).unwrap_or(true)
                && self.ecs_get::<Owner>(id).map(|o| o.house.as_ref() == house).unwrap_or(false)
                && self.ecs_get::<Identity>(id).map(|i| i.kind == MapEntityKind::Structure).unwrap_or(false)
                && self.ecs_get::<Identity>(id).map(|i| factory_matches_unit(&self.definitions, &i.type_id, kind)).unwrap_or(false)
        })
    }

    #[doc(hidden)]
    pub fn find_idle_factory(&self, house: &str, kind: TechnoClass) -> Option<usize> {
        self.entities.iter().position(|e| {
            let id = e.id;
            !self.ecs_get::<Health>(id).map(|h| h.dead).unwrap_or(true)
                && self.ecs_get::<Owner>(id).map(|o| o.house.as_ref() == house).unwrap_or(false)
                && self.ecs_get::<Identity>(id).map(|i| i.kind == MapEntityKind::Structure).unwrap_or(false)
                && self.ecs_get::<ProductionQueue>(id).map(|q| q.item.is_none() && q.ready.is_none()).unwrap_or(false)
                && self.ecs_get::<Identity>(id).map(|i| factory_matches_unit(&self.definitions, &i.type_id, kind)).unwrap_or(false)
        })
    }

    /// 本阵营建造场是否持有待放置的完工建筑。
    pub fn house_ready_building(&self, house: &str) -> Option<std::sync::Arc<str>> {
        self.entities.iter().find_map(|e| {
            let id = e.id;
            if self.ecs_get::<Health>(id).map(|h| h.dead).unwrap_or(true) {
                return None;
            }
            if !self.ecs_get::<Owner>(id).map(|o| o.house.as_ref() == house).unwrap_or(false) {
                return None;
            }
            if !self
                .ecs_get::<Identity>(id)
                .map(|i| i.kind == MapEntityKind::Structure && crate::gameplay::is_construction_yard(&self.definitions, &i.type_id))
                .unwrap_or(false)
            {
                return None;
            }
            self.ecs_get::<ProductionQueue>(id).and_then(|q| q.ready.clone())
        })
    }
}
