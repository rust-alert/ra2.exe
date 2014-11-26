//! 结构变更命令缓冲。

use crate::{Component, EcsEntity, EcsWorld};

type DeferredOp = Box<dyn FnOnce(&mut EcsWorld)>;

/// 结构变更缓冲：系统迭代期间只入队，阶段末再 [`EcsWorld::apply`]。
#[derive(Default)]
pub struct EcsCommandBuffer {
    ops: Vec<DeferredOp>,
}

impl std::fmt::Debug for EcsCommandBuffer {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("EcsCommandBuffer").field("ops", &self.ops.len()).finish()
    }
}

impl EcsCommandBuffer {
    /// 空缓冲。
    pub fn new() -> Self {
        Self::default()
    }

    /// 是否无待执行操作。
    pub fn is_empty(&self) -> bool {
        self.ops.is_empty()
    }

    /// 待执行操作数量。
    pub fn len(&self) -> usize {
        self.ops.len()
    }

    /// 延迟生成实体。
    pub fn spawn(&mut self) {
        self.ops.push(Box::new(|world| {
            let _ = world.spawn();
        }));
    }

    /// 延迟生成实体，并在生成后立刻向该实体写入组件。
    pub fn spawn_with<T: Component>(&mut self, component: T) {
        self.ops.push(Box::new(move |world| {
            let entity = world.spawn();
            world.insert(entity, component);
        }));
    }

    /// 延迟销毁实体（无效句柄在 `apply` 时被忽略）。
    pub fn despawn(&mut self, entity: EcsEntity) {
        self.ops.push(Box::new(move |world| {
            let _ = world.despawn(entity);
        }));
    }

    /// 延迟插入或覆盖组件。
    pub fn insert<T: Component>(&mut self, entity: EcsEntity, component: T) {
        self.ops.push(Box::new(move |world| {
            let _ = world.insert(entity, component);
        }));
    }

    /// 延迟移除组件。
    pub fn remove<T: Component>(&mut self, entity: EcsEntity) {
        self.ops.push(Box::new(move |world| {
            let _ = world.remove::<T>(entity);
        }));
    }

    pub(crate) fn into_ops(self) -> Vec<DeferredOp> {
        self.ops
    }
}
