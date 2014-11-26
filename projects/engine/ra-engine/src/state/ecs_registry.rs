//! `EntityId` ↔ `EcsEntity` 映射，以及内部 [`EcsWorld`]。
//!
//! 当前阶段：每个 `WorldEntity` 在 ECS 中有对应存活句柄，玩法状态仍以 `WorldEntity` 为权威。
//! tick 末把身份 / 空间 / 生命同步进组件，供迁移期只读查询。后续再翻转权威读写方向。

use std::{collections::HashMap, sync::Arc};

use ra_ecs::{EcsEntity, EcsWorld};
use ra_types::EntityId;

use super::{
    components::{Health, Identity, Owner, Transform},
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

    /// 注册句柄并写入基础组件快照。
    pub(crate) fn bind_from_world_entity(&mut self, entity: &WorldEntity) -> EcsEntity {
        let handle = self.register(entity.id);
        self.write_components(handle, entity);
        handle
    }

    /// 用 `WorldEntity` 覆盖基础组件（权威仍在 `WorldEntity`）。
    pub(crate) fn write_components(&mut self, handle: EcsEntity, entity: &WorldEntity) {
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
}
