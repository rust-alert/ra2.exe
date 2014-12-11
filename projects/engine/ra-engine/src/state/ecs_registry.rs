//! `EntityId` ↔ `EcsEntity` 映射，以及内部 [`EcsWorld`]。
//!
//! 玩法运行时组件以 ECS 为权威，经投影写回 `WorldEntity`。
//! 播种时由 `WorldEntity` 初始快照写入组件；之后 tick 只做 ECS → 投影。

use std::{collections::HashMap, sync::Arc};

use ra_ecs::{EcsEntity, EcsWorld};
use ra_types::EntityId;

use super::{
    components::{
        AnimationState, AttackState, CombatStats, HarvesterState, Health, Identity, Locomotor, MovementState, Owner,
        ProductionQueue, Transform,
    },
    entities::WorldEntity,
};

/// 一局内的 ECS 世界与对外 ID 映射。
#[derive(Debug, Clone, Default)]
pub(crate) struct EcsRegistry {
    world: EcsWorld,
    by_id: HashMap<EntityId, EcsEntity>,
}

impl EcsRegistry {
    /// 空注册表。
    pub(crate) fn new() -> Self {
        Self::default()
    }

    /// 为稳定 ID 分配 ECS 句柄（同一 `EntityId` 重复注册会替换旧句柄）。
    pub(crate) fn register(&mut self, id: EntityId) -> EcsEntity {
        if let Some(old) = self.by_id.remove(&id) {
            let _ = self.world.despawn(old);
        }
        let entity = self.world.spawn();
        self.by_id.insert(id, entity);
        entity
    }

    /// 注册句柄并写入完整组件快照（仅播种 / 生成路径）。
    pub(crate) fn bind_from_world_entity(&mut self, entity: &WorldEntity) -> EcsEntity {
        let handle = self.register(entity.id);
        self.write_all_components(handle, entity);
        handle
    }

    /// 用 `WorldEntity` 覆盖全部组件（仅绑定时使用）。
    pub(crate) fn write_all_components(&mut self, handle: EcsEntity, entity: &WorldEntity) {
        self.world.insert(
            handle,
            Identity { entity_id: entity.id, type_id: Arc::clone(&entity.type_id), kind: entity.kind },
        );
        self.world.insert(handle, Owner { house: Arc::clone(&entity.owner) });
        self.world.insert(
            handle,
            Transform {
                x: entity.x,
                y: entity.y,
                facing: entity.facing,
                turret_facing: entity.turret_facing,
                sub_cell: entity.sub_cell,
            },
        );
        self.world.insert(
            handle,
            Health { current: entity.health, maximum: entity.max_health, dead: entity.dead },
        );
        self.world.insert(handle, Locomotor { speed: entity.speed });
        self.world.insert(
            handle,
            MovementState {
                destination_x: entity.target_x,
                destination_y: entity.target_y,
                path: entity.path.clone(),
                move_accum: entity.move_accum,
            },
        );
        self.world.insert(
            handle,
            CombatStats {
                armor: entity.armor.clone(),
                attack_range: entity.attack_range,
                attack_damage: entity.attack_damage,
                attack_cooldown_max: entity.attack_cooldown_max,
                attack_verses: entity.attack_verses,
                techno_kind: entity.techno_kind,
            },
        );
        self.world.insert(
            handle,
            AttackState { target: entity.attack_target, cooldown: entity.attack_cooldown },
        );
        self.world.insert(
            handle,
            ProductionQueue {
                item: entity.produce_queue.clone(),
                rally_x: entity.rally_x,
                rally_y: entity.rally_y,
            },
        );
        self.world.insert(handle, HarvesterState { ore_trip_accum: entity.ore_trip_accum });
        self.world.insert(
            handle,
            AnimationState { hva_frame: entity.hva_frame, hit_flash: entity.hit_flash },
        );
    }

    /// 按稳定 ID 解析仍有效的内部句柄。
    pub(crate) fn resolve(&self, id: EntityId) -> Option<EcsEntity> {
        let entity = *self.by_id.get(&id)?;
        if self.world.contains(entity) { Some(entity) } else { None }
    }

    /// 已注册的稳定 ID 数量（含尚未从映射表剔除的死亡实体）。
    pub(crate) fn len(&self) -> usize {
        self.by_id.len()
    }

    /// ECS 存活实体数（应与映射表在播种/生成路径上保持一致）。
    pub(crate) fn ecs_len(&self) -> usize {
        self.world.len()
    }

    /// 只读访问内部世界。
    pub(crate) fn world(&self) -> &EcsWorld {
        &self.world
    }

    /// 可变访问内部世界。
    pub(crate) fn world_mut(&mut self) -> &mut EcsWorld {
        &mut self.world
    }
}
