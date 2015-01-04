//! `EntityId` ↔ `EcsEntity` 映射，以及内部 [`EcsWorld`]。
//!
//! 玩法组件以 ECS 为权威。投影槽 `WorldEntity` 由 `bind_bundle` 之后的
//! `project_entity_from_ecs` 覆盖。tick 只做 ECS → 投影，禁止投影回写 ECS。

use std::collections::HashMap;

use ra_ecs::{EcsEntity, EcsWorld};
use ra_types::EntityId;

use super::components::EntitySpawnBundle;

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

    /// 注册句柄并写入组件包（ECS 权威生成路径）。
    pub(crate) fn bind_bundle(&mut self, id: EntityId, bundle: EntitySpawnBundle) -> EcsEntity {
        let handle = self.register(id);
        self.world.insert(handle, bundle.identity);
        self.world.insert(handle, bundle.owner);
        self.world.insert(handle, bundle.transform);
        self.world.insert(handle, bundle.health);
        self.world.insert(handle, bundle.locomotor);
        self.world.insert(handle, bundle.movement);
        self.world.insert(handle, bundle.combat);
        self.world.insert(handle, bundle.attack);
        self.world.insert(handle, bundle.production);
        self.world.insert(handle, bundle.harvester);
        self.world.insert(handle, bundle.animation);
        handle
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
