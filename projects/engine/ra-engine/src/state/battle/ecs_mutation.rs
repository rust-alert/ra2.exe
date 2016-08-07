use super::super::{
    components::Identity,
    entities::WorldEntity,
};
use super::types::BattleState;
use crate::
spatial::is_mobile;
use ra_types::EntityId;


impl BattleState {
    /// 分配下一枚稳定实体 ID（供后续生成建筑/单位使用）。
    pub fn alloc_entity_id(&mut self) -> EntityId {
        let id = EntityId(self.next_entity_id);
        self.next_entity_id = self.next_entity_id.saturating_add(1);
        id
    }

    /// 以 ECS 组件包生成实体：占投影槽 → 写权威组件 → 投影回槽位。
    pub(crate) fn spawn_from_bundle(&mut self, bundle: crate::state::components::EntitySpawnBundle) -> usize {
        let id = bundle.identity.entity_id;
        self.entities.push(WorldEntity::projection_slot(id));
        let index = self.entities.len() - 1;
        self.ecs.bind_bundle(id, bundle);
        self.project_entity_from_ecs(id);
        index
    }

    /// 将单个实体的全部 ECS 组件投影回 `WorldEntity`。
    pub(crate) fn project_entity_from_ecs(&mut self, id: EntityId) {
        let Some(handle) = self.ecs.resolve(id)
        else {
            return;
        };
        let Some(index) = self.entity_index(id)
        else {
            return;
        };
        let identity = self.ecs.world().get::<crate::state::components::Identity>(handle).cloned();
        let owner = self.ecs.world().get::<crate::state::components::Owner>(handle).cloned();
        let health = self.ecs.world().get::<crate::state::components::Health>(handle).copied();
        let transform = self.ecs.world().get::<crate::state::components::Transform>(handle).copied();
        let loco = self.ecs.world().get::<crate::state::components::Locomotor>(handle).copied();
        let movement = self.ecs.world().get::<crate::state::components::MovementState>(handle).cloned();
        let stats = self.ecs.world().get::<crate::state::components::CombatStats>(handle).cloned();
        let attack = self.ecs.world().get::<crate::state::components::AttackState>(handle).copied();
        let queue = self.ecs.world().get::<crate::state::components::ProductionQueue>(handle).cloned();
        let harvester = self.ecs.world().get::<crate::state::components::HarvesterState>(handle).copied();
        let anim = self.ecs.world().get::<crate::state::components::AnimationState>(handle).copied();

        let entity = &mut self.entities[index];
        if let Some(identity) = identity {
            entity.type_id = identity.type_id;
            entity.kind = identity.kind;
        }
        if let Some(owner) = owner {
            entity.owner = owner.house;
        }
        if let Some(health) = health {
            entity.health = health.current;
            entity.max_health = health.maximum;
            entity.dead = health.dead;
        }
        if let Some(transform) = transform {
            entity.x = transform.x;
            entity.y = transform.y;
            entity.facing = transform.facing;
            entity.turret_facing = transform.turret_facing;
            entity.sub_cell = transform.sub_cell;
        }
        if let Some(loco) = loco {
            entity.speed = loco.speed;
        }
        if let Some(movement) = movement {
            entity.target_x = movement.destination_x;
            entity.target_y = movement.destination_y;
            entity.path = movement.path;
            entity.move_accum = movement.move_accum;
        }
        if let Some(stats) = stats {
            entity.armor = stats.armor;
            entity.attack_range = stats.attack_range;
            entity.attack_damage = stats.attack_damage;
            entity.attack_cooldown_max = stats.attack_cooldown_max;
            entity.attack_verses = stats.attack_verses;
            entity.techno_kind = stats.techno_kind;
        }
        if let Some(attack) = attack {
            entity.attack_target = attack.target;
            entity.attack_cooldown = attack.cooldown;
        }
        if let Some(queue) = queue {
            entity.produce_queue = queue.item;
            entity.rally_x = queue.rally_x;
            entity.rally_y = queue.rally_y;
        }
        if let Some(harvester) = harvester {
            entity.ore_trip_accum = harvester.ore_trip_accum;
        }
        if let Some(anim) = anim {
            entity.hva_frame = anim.hva_frame;
            entity.hit_flash = anim.hit_flash;
        }
    }

    /// 按稳定 ID 只读取 ECS 组件（玩法系统应优先走此路径，而非读投影）。
    pub(crate) fn ecs_get<T: ra_ecs::Component>(&self, id: EntityId) -> Option<&T> {
        let handle = self.ecs.resolve(id)?;
        self.ecs.world().get::<T>(handle)
    }

    /// 把 ECS 权威组件投影回 `WorldEntity`（兼容尚未改读组件的路径）。
    ///
    /// 不再从 `WorldEntity` 回写 ECS，避免双真相。
    pub(crate) fn sync_ecs_components(&mut self) {
        let ids: Vec<EntityId> = self.entities.iter().map(|e| e.id).collect();
        for id in ids {
            self.project_entity_from_ecs(id);
        }
    }

    fn with_component_mut<T: ra_ecs::Component, R>(&mut self, id: EntityId, f: impl FnOnce(&mut T) -> R) -> Option<R> {
        let handle = self.ecs.resolve(id)?;
        let result = {
            let component = self.ecs.world_mut().get_mut::<T>(handle)?;
            f(component)
        };
        self.project_entity_from_ecs(id);
        Some(result)
    }

    /// 以 ECS 为权威修改身份，并立即投影回 `WorldEntity`。
    pub(crate) fn with_identity_mut<R>(&mut self, id: EntityId, f: impl FnOnce(&mut crate::state::components::Identity) -> R) -> Option<R> {
        self.with_component_mut(id, f)
    }

    /// 以 ECS 为权威修改所属房主，并立即投影回 `WorldEntity`。
    pub(crate) fn with_owner_mut<R>(&mut self, id: EntityId, f: impl FnOnce(&mut crate::state::components::Owner) -> R) -> Option<R> {
        self.with_component_mut(id, f)
    }

    /// 以 ECS 为权威修改移动能力，并立即投影回 `WorldEntity`。
    pub(crate) fn with_locomotor_mut<R>(&mut self, id: EntityId, f: impl FnOnce(&mut crate::state::components::Locomotor) -> R) -> Option<R> {
        self.with_component_mut(id, f)
    }

    /// 以 ECS 为权威修改战斗参数，并立即投影回 `WorldEntity`。
    pub(crate) fn with_combat_stats_mut<R>(
        &mut self,
        id: EntityId,
        f: impl FnOnce(&mut crate::state::components::CombatStats) -> R,
    ) -> Option<R> {
        self.with_component_mut(id, f)
    }

    /// 以 ECS 为权威修改生命，并立即投影回 `WorldEntity`。
    pub(crate) fn with_health_mut<R>(&mut self, id: EntityId, f: impl FnOnce(&mut crate::state::components::Health) -> R) -> Option<R> {
        self.with_component_mut(id, f)
    }

    /// 以 ECS 为权威修改空间变换，并立即投影回 `WorldEntity`。
    pub(crate) fn with_transform_mut<R>(&mut self, id: EntityId, f: impl FnOnce(&mut crate::state::components::Transform) -> R) -> Option<R> {
        self.with_component_mut(id, f)
    }

    /// 以 ECS 为权威修改移动状态，并立即投影回 `WorldEntity`。
    pub(crate) fn with_movement_mut<R>(
        &mut self,
        id: EntityId,
        f: impl FnOnce(&mut crate::state::components::MovementState) -> R,
    ) -> Option<R> {
        self.with_component_mut(id, f)
    }

    /// 为指定下标实体重算路径并写入 ECS `MovementState`。
    pub(crate) fn repath_entity_at(&mut self, index: usize) {
        let path = self.compute_repath_at(index);
        let id = self.entities[index].id;
        let _ = self.with_movement_mut(id, |movement| {
            movement.path = path;
        });
    }

    /// 以 ECS 为权威修改攻击状态，并立即投影回 `WorldEntity`。
    pub(crate) fn with_attack_mut<R>(&mut self, id: EntityId, f: impl FnOnce(&mut crate::state::components::AttackState) -> R) -> Option<R> {
        self.with_component_mut(id, f)
    }

    /// 以 ECS 为权威修改生产队列，并立即投影回 `WorldEntity`。
    pub(crate) fn with_production_mut<R>(
        &mut self,
        id: EntityId,
        f: impl FnOnce(&mut crate::state::components::ProductionQueue) -> R,
    ) -> Option<R> {
        self.with_component_mut(id, f)
    }

    /// 以 ECS 为权威修改采矿行程，并立即投影回 `WorldEntity`。
    pub(crate) fn with_harvester_mut<R>(
        &mut self,
        id: EntityId,
        f: impl FnOnce(&mut crate::state::components::HarvesterState) -> R,
    ) -> Option<R> {
        self.with_component_mut(id, f)
    }

    /// 以 ECS 为权威修改动画桥接状态，并立即投影回 `WorldEntity`。
    pub(crate) fn with_animation_mut<R>(
        &mut self,
        id: EntityId,
        f: impl FnOnce(&mut crate::state::components::AnimationState) -> R,
    ) -> Option<R> {
        self.with_component_mut(id, f)
    }

    /// 测试 / 调试：写入 ECS 炮塔朝向并投影。
    pub fn set_ecs_turret_facing(&mut self, id: EntityId, turret_facing: u8) -> bool {
        self.with_transform_mut(id, |transform| {
            transform.turret_facing = turret_facing;
        })
            .is_some()
    }

    /// 测试 / 调试：写入 ECS `Health` 并投影。
    pub fn set_ecs_health(&mut self, id: EntityId, current: u32, maximum: u32, dead: bool) -> bool {
        self.with_health_mut(id, |health| {
            health.current = current;
            health.maximum = maximum;
            health.dead = dead;
        })
            .is_some()
    }

    /// 测试 / 调试：写入 ECS 攻击参数并投影。
    pub fn set_ecs_attack_power(&mut self, id: EntityId, damage: u32, range: u32, cooldown_max: u32) -> bool {
        self.with_combat_stats_mut(id, |stats| {
            stats.attack_damage = damage;
            stats.attack_range = range;
            stats.attack_cooldown_max = cooldown_max;
        })
            .is_some()
    }

    /// 测试 / 调试：写入 ECS 身份类型并投影。
    pub fn set_ecs_type_id(&mut self, id: EntityId, type_id: impl Into<std::sync::Arc<str>>, kind: ra_map::MapEntityKind) -> bool {
        let type_id = type_id.into();
        self.with_identity_mut(id, |identity| {
            identity.type_id = type_id;
            identity.kind = kind;
        })
            .is_some()
    }

    /// 测试 / 调试：写入 ECS 移动速度并投影。
    pub fn set_ecs_speed(&mut self, id: EntityId, speed: u32) -> bool {
        self.with_locomotor_mut(id, |loco| {
            loco.speed = speed;
        })
            .is_some()
    }

    /// 测试 / 调试：清空 ECS 移动目的地与路径并投影。
    pub fn clear_ecs_movement(&mut self, id: EntityId) -> bool {
        self.with_movement_mut(id, |movement| {
            movement.destination_x = None;
            movement.destination_y = None;
            movement.waypoints.clear();
            movement.path.clear();
            movement.move_accum = 0;
        })
            .is_some()
    }

    /// 通行表变更后，为全部移动单位重算路径。
    pub fn repath_mobiles(&mut self) {
        use crate::state::components::Identity;

        for i in 0..self.entities.len() {
            let id = self.entities[i].id;
            if self.ecs_get::<Identity>(id).map(|identity| is_mobile(identity.kind)).unwrap_or(false) {
                self.repath_entity_at(i);
            }
        }
        self.rehash();
    }
}
