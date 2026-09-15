//! 工厂生产队列、出厂与集结。

use ra_map::MapEntityKind;
use ra_types::{EntityId, TechnoClass, TechnoDefinition, TypeId};

use crate::{
    gameplay::{factory_matches_unit, is_construction_yard, structure_is_defense, verses_for},
    state::{
        ATTACK_COOLDOWN_TICKS, BUILD_TIME_TICKS_PER_UNIT, PRODUCE_TICKS,
        components::{
            AnimationState, AttackState, CashProducerState, CombatStats, DeployStance, EntitySpawnBundle, HarvesterState, Health, Identity,
            Locomotor, MovementState, Owner, ProductionQueue, ProductionSlot, Transform,
        },
    },
};

/// 将冻结定义中的 `BuildTime` 转为生产队列剩余 tick。
///
/// `build_time == 0`（缺省）回退 [`PRODUCE_TICKS`]。否则为 `build_time * BUILD_TIME_TICKS_PER_UNIT`，至少 1。
pub fn produce_ticks_for(techno: &TechnoDefinition) -> u32 {
    if techno.build_time == 0 { PRODUCE_TICKS } else { techno.build_time.saturating_mul(BUILD_TIME_TICKS_PER_UNIT).max(1) }
}

/// 下一步推进应付金额：按已完成比例对齐 `Cost`，末步补齐差额。没钱则不应调用方推进。
pub fn produce_step_due(cost: i32, total_ticks: u32, remaining_ticks: u32, paid: i32) -> i32 {
    if cost <= 0 || total_ticks == 0 || remaining_ticks == 0 {
        return 0;
    }
    let completed_after = total_ticks.saturating_sub(remaining_ticks).saturating_add(1);
    let target = if remaining_ticks <= 1 || completed_after >= total_ticks {
        cost
    }
    else {
        ((i64::from(cost) * i64::from(completed_after)) / i64::from(total_ticks)) as i32
    };
    (target - paid).max(0)
}

impl crate::state::BattleState {
    #[doc(hidden)]
    pub fn advance_production(&mut self) {
        let mut unit_spawns: Vec<(usize, TypeId)> = Vec::new();
        let mut building_ready: Vec<(usize, TypeId, bool)> = Vec::new();
        let mut unit_promote: Vec<(usize, TypeId)> = Vec::new();
        let n = self.entities.len();
        for index in 0..n {
            let id = self.entities[index].id;
            if self.ecs_get::<Health>(id).map(|h| h.dead).unwrap_or(true) {
                continue;
            }
            let low_power = self
                .ecs_get::<Owner>(id)
                .and_then(|o| self.players.iter().find(|p| p.house_id == Some(o.house)))
                .is_some_and(|p| p.low_power());
            let mut finished: Vec<(TypeId, bool)> = Vec::new();
            if let Some(type_id) = self.try_advance_factory_slot(id, false, low_power) {
                finished.push((type_id, false));
            }
            if let Some(type_id) = self.try_advance_factory_slot(id, true, low_power) {
                finished.push((type_id, true));
            }
            for (type_id, defense) in finished {
                let is_building = self.definitions.techno.get_by_id(type_id).is_some_and(|t| t.class == TechnoClass::Building);
                if is_building {
                    building_ready.push((index, type_id, defense));
                }
                else {
                    unit_spawns.push((index, type_id));
                }
            }
        }
        for (factory_index, type_id, defense) in building_ready {
            let factory_id = self.entities[factory_index].id;
            let _ = self.with_production_mut(factory_id, |queue| {
                if defense {
                    queue.defense_ready = Some(type_id);
                }
                else {
                    queue.ready = Some(type_id);
                }
            });
            self.mark_entity_dirty(factory_id);
            if let Some(owner) = self
                .ecs_get::<Owner>(factory_id)
                .map(|o| std::sync::Arc::<str>::from(crate::gameplay::house_key_of(&self.definitions, o.house)))
            {
                self.push_eva_cue(owner.as_ref(), "EVA_ConstructionComplete");
            }
        }
        for (factory_index, type_id) in unit_spawns {
            self.spawn_produced_unit(factory_index, type_id);
            let factory_id = self.entities[factory_index].id;
            if let Some(next) = self.with_production_mut(factory_id, |queue| queue.pending.first().copied()).flatten() {
                unit_promote.push((factory_index, next));
            }
        }
        for (factory_index, next_id) in unit_promote {
            let factory_id = self.entities[factory_index].id;
            let Some(ticks) = self.definitions.techno.get_by_id(next_id).map(produce_ticks_for)
            else {
                let _ = self.with_production_mut(factory_id, |queue| {
                    if !queue.pending.is_empty() {
                        queue.pending.remove(0);
                    }
                });
                continue;
            };
            let _ = self.with_production_mut(factory_id, |queue| {
                if queue.item.is_some() {
                    return;
                }
                if queue.pending.first().copied() != Some(next_id) {
                    return;
                }
                queue.pending.remove(0);
                queue.item = Some(ProductionSlot::new(next_id, ticks));
            });
            self.mark_entity_dirty(factory_id);
        }
    }

    /// 推进工厂单轨：边造边扣；资金不足则暂停。返回本 tick 完工的类型。
    fn try_advance_factory_slot(&mut self, factory_id: EntityId, defense: bool, low_power: bool) -> Option<TypeId> {
        // 低电：每隔一 tick 才推进，等效半速（与供电不足反馈一致）。
        if low_power && self.tick % 2 == 1 {
            return None;
        }
        let owner_house = self.ecs_get::<Owner>(factory_id).map(|o| o.house)?;
        let house_key = crate::gameplay::house_key_of(&self.definitions, owner_house);
        let player_index =
            self.players.iter().position(|p| p.house_id == Some(owner_house) || p.house.as_ref().eq_ignore_ascii_case(house_key))?;
        let snapshot = self.ecs_get::<ProductionQueue>(factory_id).and_then(|q| {
            let slot = if defense { q.defense_item.as_ref() } else { q.item.as_ref() }?;
            Some((slot.type_id, slot.remaining_ticks, slot.total_ticks, slot.paid))
        })?;
        let (type_id, remaining, total, paid) = snapshot;
        let cost = self.definitions.techno.get_by_id(type_id).map(|t| t.cost).unwrap_or(0);
        let due = produce_step_due(cost, total, remaining, paid);
        if due > 0 && self.players[player_index].funds < due {
            return None;
        }
        let finished = self
            .with_production_mut(factory_id, |queue| {
                let slot_opt = if defense { &mut queue.defense_item } else { &mut queue.item };
                let Some(slot) = slot_opt.as_mut()
                else {
                    return None;
                };
                if slot.type_id != type_id || slot.remaining_ticks != remaining {
                    return None;
                }
                slot.paid = slot.paid.saturating_add(due);
                if slot.remaining_ticks > 1 {
                    slot.remaining_ticks -= 1;
                    return Some(false);
                }
                *slot_opt = None;
                Some(true)
            })
            .flatten()?;
        if due > 0 {
            self.players[player_index].funds -= due;
            self.players[player_index].funds_spent = self.players[player_index].funds_spent.saturating_add(due);
        }
        finished.then_some(type_id)
    }

    #[doc(hidden)]
    pub fn spawn_produced_unit(&mut self, factory_index: usize, type_id: TypeId) {
        let Some(tt) = self.definitions.techno.get_by_id(type_id).cloned()
        else {
            return;
        };
        let factory_id = self.entities[factory_index].id;
        let Some(owner_id) = self.ecs_get::<Owner>(factory_id).map(|o| o.house)
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
        let Some((x, y)) = self.find_spawn_cell(factory_xf.x, factory_xf.y, factory_id, tt.naval)
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
        let promoted = self.players.iter().find(|p| p.house_id == Some(owner_id)).is_some_and(|p| match tt.class {
            TechnoClass::Infantry => p.promoted_infantry,
            TechnoClass::Vehicle => p.promoted_vehicle,
            _ => false,
        });
        let base_health = tt.strength.max(1);
        let max_health =
            if promoted { base_health.saturating_mul(5).saturating_div(4).max(base_health.saturating_add(1)) } else { base_health };
        let def_id = tt.id;
        let sight = tt.sight;
        let warhead_fallback = tt.warhead_id;
        let weapon = tt.primary_id.and_then(|id| self.definitions.weapons.get_by_id(id));
        let attack_range = weapon.map(|w| if w.range > 0 { w.range } else { sight.max(1) }).unwrap_or(0);
        // 无 Primary / Damage=0 保持 0，禁止用 Strength 发明伤害。
        let attack_damage = weapon.map(|w| w.damage).unwrap_or(0);
        let attack_cooldown_max = weapon.map(|w| if w.rof > 0 { w.rof } else { ATTACK_COOLDOWN_TICKS }).unwrap_or(0);
        let warhead_id = weapon.and_then(|w| w.warhead_id).or(warhead_fallback);
        let attack_verses = verses_for(&self.definitions, warhead_id);
        let id = self.alloc_entity_id();
        let house_key = self.definitions.houses.get_by_id(owner_id).map(|h| h.type_key.as_str().to_string());
        let local_house = self.players.iter().find(|p| p.id == self.local_player).map(|p| p.house.clone());
        let is_ai_house = house_key.as_ref().is_some_and(|house| {
            !crate::gameplay::ai::is_ambient_house(house)
                && local_house.as_ref().map(|h| !h.as_ref().eq_ignore_ascii_case(house)).unwrap_or(true)
        });
        let guard_by_iq = is_ai_house
            && house_key.as_ref().is_some_and(|house| {
                !tt.harvester
                    && crate::gameplay::ai::iq_allows(self, house, self.definitions.ai_controls.iq_guard_area)
                    && matches!(kind, MapEntityKind::Unit | MapEntityKind::Infantry | MapEntityKind::Aircraft)
            });
        let mission = if tt.harvester {
            Some(ra_types::MissionKind::Harvest)
        } else if guard_by_iq {
            Some(ra_types::MissionKind::Guard)
        } else {
            None
        };
        let unit_index = self.spawn_from_bundle(EntitySpawnBundle {
            identity: Identity { entity_id: id, type_id: def_id, kind, mission, tag: None },
            owner: Owner { house: owner_id },
            transform: Transform { x, y, facing: 0, turret_facing: 0, sub_cell: 0 },
            health: Health { current: max_health, maximum: max_health, dead: false },
            locomotor: Locomotor { speed: tt.speed },
            movement: MovementState { destination_x: None, destination_y: None, waypoints: Vec::new(), path: Vec::new(), move_accum: 0 },
            combat: CombatStats {
                armor: tt.armor,
                attack_range,
                attack_damage,
                attack_cooldown_max,
                attack_verses,
                techno_class: Some(techno_class),
            },
            attack: AttackState { target: None, cooldown: 0, infiltrate_target: None, capture_target: None, follow_target: None },
            production: ProductionQueue::empty(),
            harvester: HarvesterState { ore_trip_accum: 0, cargo: 0 },
            cash_producer: CashProducerState::default(),
            animation: AnimationState { hva_frame: 0, hit_flash: 0, fire_flash: 0 },
            deploy_stance: DeployStance { deployed: false },
        });
        self.mark_entity_dirty(id);
        if let Some(player) =
            self.players.iter_mut().find(|p| crate::gameplay::house_id_of(&self.definitions, p.house.as_ref()) == Some(owner_id))
        {
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
    pub fn find_spawn_cell(&self, fx: u16, fy: u16, factory_id: ra_types::EntityId, naval: bool) -> Option<(u16, u16)> {
        if naval {
            return self.find_naval_spawn_cell(fx, fy, factory_id);
        }
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

    /// 在工厂 `Foundation` 外沿找空水面格（船厂出舰）。
    fn find_naval_spawn_cell(&self, fx: u16, fy: u16, factory_id: ra_types::EntityId) -> Option<(u16, u16)> {
        let foundation = self
            .ecs_get::<Identity>(factory_id)
            .and_then(|i| self.definitions.structures.get_by_id(i.type_id))
            .map(|s| s.foundation.clone())
            .unwrap_or_default();
        let fw = i32::from(foundation.width.max(1));
        let fh = i32::from(foundation.height.max(1));
        const MAX_RADIUS: i32 = 8;
        for radius in 1..=MAX_RADIUS {
            for dy in -radius..=(fh - 1 + radius) {
                for dx in -radius..=(fw - 1 + radius) {
                    let on_ring = dx == -radius || dy == -radius || dx == fw - 1 + radius || dy == fh - 1 + radius;
                    if !on_ring {
                        continue;
                    }
                    let x = i32::from(fx) + dx;
                    let y = i32::from(fy) + dy;
                    if x < 0 || y < 0 {
                        continue;
                    }
                    let (x, y) = (x as u16, y as u16);
                    if !self.pass_grid.is_naval_passable(x, y) {
                        continue;
                    }
                    if self.cell_blocked_by_entity(x, y) {
                        continue;
                    }
                    return Some((x, y));
                }
            }
        }
        None
    }

    #[doc(hidden)]
    pub fn find_factory(&self, house: &str, kind: TechnoClass) -> Option<usize> {
        self.entities.iter().position(|e| {
            let id = e.id;
            !self.ecs_get::<Health>(id).map(|h| h.dead).unwrap_or(true)
                && self.ecs_get::<Owner>(id).map(|o| crate::gameplay::house_id_of(&self.definitions, house) == Some(o.house)).unwrap_or(false)
                && self.ecs_get::<Identity>(id).map(|i| i.kind == MapEntityKind::Structure).unwrap_or(false)
                && self.ecs_get::<Identity>(id).map(|i| factory_matches_unit(&self.definitions, i.type_id, kind)).unwrap_or(false)
        })
    }

    /// 建造场对应结构轨是否空闲（建筑栏 / 防御栏分轨，可并发）。
    #[doc(hidden)]
    pub fn find_idle_structure_yard(&self, house: &str, defense: bool) -> Option<usize> {
        self.entities.iter().position(|e| {
            let id = e.id;
            !self.ecs_get::<Health>(id).map(|h| h.dead).unwrap_or(true)
                && self.ecs_get::<Owner>(id).map(|o| crate::gameplay::house_id_of(&self.definitions, house) == Some(o.house)).unwrap_or(false)
                && self
                    .ecs_get::<Identity>(id)
                    .map(|i| i.kind == MapEntityKind::Structure && is_construction_yard(&self.definitions, i.type_id))
                    .unwrap_or(false)
                && self.ecs_get::<ProductionQueue>(id).map(|q| !q.structure_track_busy(defense)).unwrap_or(false)
        })
    }

    /// 单位厂：优先主厂（PRI）；否则空闲厂；再否则最短仍可入队的 FIFO 厂。
    #[doc(hidden)]
    pub fn find_unit_factory_for_enqueue(&self, house: &str, kind: TechnoClass) -> Option<usize> {
        if matches!(kind, TechnoClass::Building) {
            return None;
        }
        let house_id = crate::gameplay::house_id_of(&self.definitions, house);
        let mut primary_idle: Option<usize> = None;
        let mut primary_busy: Option<usize> = None;
        let mut idle: Option<usize> = None;
        let mut best: Option<(usize, usize)> = None;
        for (index, e) in self.entities.iter().enumerate() {
            let id = e.id;
            if self.ecs_get::<Health>(id).map(|h| h.dead).unwrap_or(true) {
                continue;
            }
            if !self.ecs_get::<Owner>(id).map(|o| house_id == Some(o.house)).unwrap_or(false) {
                continue;
            }
            if !self.ecs_get::<Identity>(id).map(|i| i.kind == MapEntityKind::Structure).unwrap_or(false) {
                continue;
            }
            if !self.ecs_get::<Identity>(id).map(|i| factory_matches_unit(&self.definitions, i.type_id, kind)).unwrap_or(false) {
                continue;
            }
            let Some(queue) = self.ecs_get::<ProductionQueue>(id)
            else {
                continue;
            };
            if !queue.can_enqueue_unit() {
                continue;
            }
            if queue.is_primary {
                if queue.item.is_none() {
                    primary_idle = Some(index);
                }
                else {
                    primary_busy = Some(index);
                }
                continue;
            }
            let len = queue.unit_len();
            if queue.item.is_none() {
                if idle.is_none() {
                    idle = Some(index);
                }
                continue;
            }
            match best {
                Some((_, best_len)) if len >= best_len => {}
                _ => best = Some((index, len)),
            }
        }
        primary_idle.or(primary_busy).or(idle).or(best.map(|(index, _)| index))
    }

    /// 若 `id` 为生产厂且本房主该生产类别尚无主厂，则将其标为 PRI。
    pub(crate) fn maybe_assign_primary_factory(&mut self, id: EntityId) {
        let Some(identity) = self.ecs_get::<Identity>(id).cloned()
        else {
            return;
        };
        if identity.kind != MapEntityKind::Structure {
            return;
        }
        if self.ecs_get::<Health>(id).map(|h| h.dead).unwrap_or(true) {
            return;
        }
        if !crate::gameplay::is_production_factory(&self.definitions, identity.type_id) {
            return;
        }
        let Some(owner) = self.ecs_get::<Owner>(id).map(|o| o.house)
        else {
            return;
        };
        let Some(category) = self.definitions.structures.get_by_id(identity.type_id).and_then(|s| s.production.as_ref()).map(|p| p.category)
        else {
            return;
        };
        let has_primary = self.entities.iter().any(|e| {
            let eid = e.id;
            if eid == id || self.ecs_get::<Health>(eid).map(|h| h.dead).unwrap_or(true) {
                return false;
            }
            if !self.ecs_get::<Owner>(eid).is_some_and(|o| o.house == owner) {
                return false;
            }
            if !self.ecs_get::<Identity>(eid).is_some_and(|i| {
                i.kind == MapEntityKind::Structure && crate::gameplay::factory_matches_category(&self.definitions, i.type_id, category)
            }) {
                return false;
            }
            self.ecs_get::<ProductionQueue>(eid).is_some_and(|q| q.is_primary)
        });
        if has_primary {
            return;
        }
        let _ = self.with_production_mut(id, |queue| {
            queue.is_primary = true;
        });
        self.mark_entity_dirty(id);
    }

    /// 主厂被毁后，将 PRI 转给同房主同生产类别的下一座存活厂。
    pub(crate) fn reassign_primary_after_factory_lost(&mut self, lost_id: EntityId) {
        let Some(identity) = self.ecs_get::<Identity>(lost_id).cloned()
        else {
            return;
        };
        if !crate::gameplay::is_production_factory(&self.definitions, identity.type_id) {
            return;
        }
        let was_primary = self.ecs_get::<ProductionQueue>(lost_id).is_some_and(|q| q.is_primary);
        let _ = self.with_production_mut(lost_id, |queue| {
            queue.is_primary = false;
        });
        if !was_primary {
            return;
        }
        let Some(owner) = self.ecs_get::<Owner>(lost_id).map(|o| o.house)
        else {
            return;
        };
        let Some(category) = self.definitions.structures.get_by_id(identity.type_id).and_then(|s| s.production.as_ref()).map(|p| p.category)
        else {
            return;
        };
        let successor = self.entities.iter().find_map(|e| {
            let eid = e.id;
            if eid == lost_id || self.ecs_get::<Health>(eid).map(|h| h.dead).unwrap_or(true) {
                return None;
            }
            if !self.ecs_get::<Owner>(eid).is_some_and(|o| o.house == owner) {
                return None;
            }
            if !self.ecs_get::<Identity>(eid).is_some_and(|i| {
                i.kind == MapEntityKind::Structure && crate::gameplay::factory_matches_category(&self.definitions, i.type_id, category)
            }) {
                return None;
            }
            Some(eid)
        });
        if let Some(next) = successor {
            let _ = self.with_production_mut(next, |queue| {
                queue.is_primary = true;
            });
            self.mark_entity_dirty(next);
        }
        self.mark_entity_dirty(lost_id);
    }

    /// 将 `factory` 设为本房主、本生产类别唯一主厂。
    pub(crate) fn set_primary_factory(&mut self, factory: EntityId) -> bool {
        let Some(identity) = self.ecs_get::<Identity>(factory).cloned()
        else {
            return false;
        };
        if identity.kind != MapEntityKind::Structure || self.ecs_get::<Health>(factory).map(|h| h.dead).unwrap_or(true) {
            return false;
        }
        if !crate::gameplay::is_production_factory(&self.definitions, identity.type_id) {
            return false;
        }
        let Some(owner) = self.ecs_get::<Owner>(factory).map(|o| o.house)
        else {
            return false;
        };
        let Some(category) = self.definitions.structures.get_by_id(identity.type_id).and_then(|s| s.production.as_ref()).map(|p| p.category)
        else {
            return false;
        };
        let peers: Vec<EntityId> = self
            .entities
            .iter()
            .filter_map(|e| {
                let eid = e.id;
                if self.ecs_get::<Health>(eid).map(|h| h.dead).unwrap_or(true) {
                    return None;
                }
                if !self.ecs_get::<Owner>(eid).is_some_and(|o| o.house == owner) {
                    return None;
                }
                if !self.ecs_get::<Identity>(eid).is_some_and(|i| {
                    i.kind == MapEntityKind::Structure && crate::gameplay::factory_matches_category(&self.definitions, i.type_id, category)
                }) {
                    return None;
                }
                Some(eid)
            })
            .collect();
        for eid in peers {
            let _ = self.with_production_mut(eid, |queue| {
                queue.is_primary = eid == factory;
            });
            self.mark_entity_dirty(eid);
        }
        true
    }

    /// 兼容旧调用：结构轨查空闲建造场；单位查可入队厂。
    #[doc(hidden)]
    pub fn find_idle_factory(&self, house: &str, kind: TechnoClass) -> Option<usize> {
        if kind == TechnoClass::Building {
            self.find_idle_structure_yard(house, false)
        }
        else {
            self.find_unit_factory_for_enqueue(house, kind).filter(|&index| {
                let id = self.entities[index].id;
                self.ecs_get::<ProductionQueue>(id).is_some_and(|q| q.item.is_none())
            })
        }
    }

    /// 本阵营建造场是否持有待放置的完工建筑（任一轨；多轨并存时优先建筑栏）。
    pub fn house_ready_building(&self, house: &str) -> Option<TypeId> {
        self.entities.iter().find_map(|e| {
            let id = e.id;
            if self.ecs_get::<Health>(id).map(|h| h.dead).unwrap_or(true) {
                return None;
            }
            if !self.ecs_get::<Owner>(id).map(|o| crate::gameplay::house_id_of(&self.definitions, house) == Some(o.house)).unwrap_or(false) {
                return None;
            }
            if !self
                .ecs_get::<Identity>(id)
                .map(|i| i.kind == MapEntityKind::Structure && is_construction_yard(&self.definitions, i.type_id))
                .unwrap_or(false)
            {
                return None;
            }
            let queue = self.ecs_get::<ProductionQueue>(id)?;
            queue.ready.or(queue.defense_ready)
        })
    }

    /// 本阵营建造场是否持有指定类型的待落位完工件。
    pub fn house_has_ready_building(&self, house: &str, type_id: TypeId) -> bool {
        self.find_yard_with_ready(house, type_id).is_some()
    }

    /// 找到持有指定待落位完工件的建造场实体。
    pub fn find_yard_with_ready(&self, house: &str, type_id: TypeId) -> Option<ra_types::EntityId> {
        self.entities.iter().find_map(|e| {
            let id = e.id;
            if self.ecs_get::<Health>(id).map(|h| h.dead).unwrap_or(true) {
                return None;
            }
            if !self.ecs_get::<Owner>(id).map(|o| crate::gameplay::house_id_of(&self.definitions, house) == Some(o.house)).unwrap_or(false) {
                return None;
            }
            if !self
                .ecs_get::<Identity>(id)
                .map(|i| i.kind == MapEntityKind::Structure && is_construction_yard(&self.definitions, i.type_id))
                .unwrap_or(false)
            {
                return None;
            }
            let queue = self.ecs_get::<ProductionQueue>(id)?;
            (queue.ready == Some(type_id) || queue.defense_ready == Some(type_id)).then_some(id)
        })
    }

    /// 解析建筑开单应走的结构轨（防御 / 建筑）。
    pub(crate) fn structure_produce_defense_track(&self, type_id: TypeId) -> bool {
        structure_is_defense(&self.definitions, type_id)
    }
}
