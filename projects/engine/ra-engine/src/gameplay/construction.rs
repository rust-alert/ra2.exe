//! 建造放置与占地规则；建筑 `FreeUnit=` 落成白送。

use ra_map::MapEntityKind;
use ra_types::{EntityId, MissionKind, TechnoClass, TypeId};

use crate::{
    gameplay::verses_for,
    state::{
        ATTACK_COOLDOWN_TICKS,
        components::{
            AnimationState, AttackState, CombatStats, EntitySpawnBundle, HarvesterState, Health, Identity, Locomotor, MovementState, Owner,
            ProductionQueue, Transform,
        },
    },
};

impl crate::state::BattleState {
    /// 建筑落位后按 `StructureDefinition::free_unit` 白送一辆单位。
    ///
    /// 首选占地中心南一格；失败则在建筑附近搜空格。全部失败则退还该单位 `Cost`，不留实体。
    pub(crate) fn spawn_structure_free_unit(&mut self, building_id: EntityId, building_type: TypeId, house: &str, bx: u16, by: u16) {
        let Some(free_unit_id) = self.definitions.structures.get_by_id(building_type).and_then(|s| s.free_unit)
        else {
            return;
        };
        let Some(tt) = self.definitions.techno.get_by_id(free_unit_id).cloned()
        else {
            return;
        };
        let foundation = self.definitions.structures.get_by_id(building_type).map(|s| s.foundation.clone()).unwrap_or_default();
        let refund = tt.cost.max(0) as i32;
        let Some((sx, sy)) = self.find_free_unit_cell(bx, by, foundation.width, foundation.height)
        else {
            self.refund_house_funds(house, refund);
            return;
        };
        let kind = match tt.class {
            TechnoClass::Infantry => MapEntityKind::Infantry,
            TechnoClass::Vehicle => MapEntityKind::Unit,
            TechnoClass::Aircraft => MapEntityKind::Aircraft,
            TechnoClass::Building => {
                self.refund_house_funds(house, refund);
                return;
            }
        };
        let Some(owner_house) = crate::gameplay::house_id_of(&self.definitions, house)
        else {
            self.refund_house_funds(house, refund);
            return;
        };
        let techno_class = tt.class;
        let max_health = tt.strength.max(1);
        let def_id = tt.id;
        let sight = tt.sight;
        let warhead_fallback = tt.warhead_id;
        let weapon = tt.primary_id.and_then(|id| self.definitions.weapons.get_by_id(id));
        let attack_range = weapon.map(|w| if w.range > 0 { w.range } else { sight.max(1) }).unwrap_or(0);
        let attack_damage = weapon.map(|w| w.damage).unwrap_or(0);
        let attack_cooldown_max = weapon.map(|w| if w.rof > 0 { w.rof } else { ATTACK_COOLDOWN_TICKS }).unwrap_or(0);
        let warhead_id = weapon.and_then(|w| w.warhead_id).or(warhead_fallback);
        let attack_verses = verses_for(&self.definitions, warhead_id);
        let mission = if tt.harvester { Some(MissionKind::Harvest) } else { None };
        let id = self.alloc_entity_id();
        self.spawn_from_bundle(EntitySpawnBundle {
            identity: Identity { entity_id: id, type_id: def_id, kind, mission, tag: None },
            owner: Owner { house: owner_house },
            transform: Transform { x: sx, y: sy, facing: 0, turret_facing: 0, sub_cell: 0 },
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
            animation: AnimationState { hva_frame: 0, hit_flash: 0, fire_flash: 0 },
        });
        self.mark_entity_dirty(id);
        if let Some(player) = self.players.iter_mut().find(|p| p.house.as_ref() == house) {
            player.built = player.built.saturating_add(1);
        }
        let _ = building_id;
    }

    fn refund_house_funds(&mut self, house: &str, amount: i32) {
        if amount <= 0 {
            return;
        }
        if let Some(player) = self.players.iter_mut().find(|p| p.house.as_ref() == house) {
            player.funds = player.funds.saturating_add(amount);
        }
    }

    /// 首选占地中心南一格；否则从建筑西北角起扩环搜可放格（避开已封占地）。
    fn find_free_unit_cell(&self, bx: u16, by: u16, width: u16, height: u16) -> Option<(u16, u16)> {
        let width = width.max(1);
        let height = height.max(1);
        let primary_x = bx.saturating_add(width / 2);
        let primary_y = by.saturating_add(height / 2).saturating_add(1);
        if self.can_place_structure(primary_x, primary_y) {
            return Some((primary_x, primary_y));
        }
        // 扩环：半径 1..=max(占地边长+8, 12)，覆盖舱位被封后的邻近空地。
        let max_r = u32::from(width.saturating_add(height)).saturating_add(8).max(12);
        for r in 1..=max_r {
            let r_i = r as i32;
            for dy in -r_i..=r_i {
                for dx in -r_i..=r_i {
                    if dx.abs().max(dy.abs()) != r_i {
                        continue;
                    }
                    let x = i32::from(bx) + dx;
                    let y = i32::from(by) + dy;
                    if x < 0 || y < 0 {
                        continue;
                    }
                    let (x, y) = (x as u16, y as u16);
                    if self.can_place_structure(x, y) {
                        return Some((x, y));
                    }
                }
            }
        }
        None
    }
}
