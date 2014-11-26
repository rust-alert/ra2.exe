//! `EcsWorld`：实体分配与组件存储。

use std::{any::TypeId, collections::HashMap};

use crate::{
    Component, EcsCommandBuffer, EcsEntity,
    entity::EntityMeta,
    store::{ComponentStore, ErasedStore, store_type_id},
};

/// 组件存储世界。
#[derive(Default)]
pub struct EcsWorld {
    metas: Vec<EntityMeta>,
    free: Vec<u32>,
    stores: HashMap<TypeId, Box<dyn ErasedStore>>,
    alive_count: u32,
}

impl std::fmt::Debug for EcsWorld {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("EcsWorld")
            .field("alive", &self.alive_count)
            .field("slots", &self.metas.len())
            .field("free", &self.free.len())
            .field("stores", &self.stores.len())
            .finish()
    }
}

impl Clone for EcsWorld {
    fn clone(&self) -> Self {
        Self {
            metas: self.metas.clone(),
            free: self.free.clone(),
            stores: self.stores.iter().map(|(k, v)| (*k, v.clone_box())).collect(),
            alive_count: self.alive_count,
        }
    }
}

impl EcsWorld {
    /// 空世界。
    pub fn new() -> Self {
        Self::default()
    }

    /// 当前存活实体数量。
    pub fn len(&self) -> usize {
        self.alive_count as usize
    }

    /// 是否没有任何存活实体。
    pub fn is_empty(&self) -> bool {
        self.alive_count == 0
    }

    /// 句柄是否仍指向存活实体。
    pub fn contains(&self, entity: EcsEntity) -> bool {
        match self.metas.get(entity.slot as usize) {
            Some(meta) => meta.alive && meta.generation == entity.generation,
            None => false,
        }
    }

    /// 立即分配存活实体。
    pub fn spawn(&mut self) -> EcsEntity {
        if let Some(slot) = self.free.pop() {
            let meta = &mut self.metas[slot as usize];
            debug_assert!(!meta.alive);
            meta.alive = true;
            self.alive_count = self.alive_count.saturating_add(1);
            EcsEntity { slot, generation: meta.generation }
        }
        else {
            let slot = self.metas.len() as u32;
            self.metas.push(EntityMeta { generation: 1, alive: true });
            self.alive_count = self.alive_count.saturating_add(1);
            EcsEntity { slot, generation: 1 }
        }
    }

    /// 立即销毁实体并移除其全部组件；旧句柄随即失效。
    pub fn despawn(&mut self, entity: EcsEntity) -> bool {
        if !self.contains(entity) {
            return false;
        }
        for store in self.stores.values_mut() {
            store.clear_slot(entity.slot);
        }
        let meta = &mut self.metas[entity.slot as usize];
        meta.alive = false;
        meta.generation = meta.generation.wrapping_add(1);
        self.free.push(entity.slot);
        self.alive_count = self.alive_count.saturating_sub(1);
        true
    }

    /// 插入或覆盖组件。
    ///
    /// 返回被覆盖的旧值。实体无效时返回 `None` 且不写入。
    pub fn insert<T: Component>(&mut self, entity: EcsEntity, component: T) -> Option<T> {
        if !self.contains(entity) {
            return None;
        }
        let store = self.ensure_store::<T>();
        // `ComponentStore::insert` 在新建时返回 `None`，覆盖时返回 `Some(old)`。
        // 对外统一：成功写入时，无旧值也视为“没有可返回的旧组件”，故直接透传。
        store.insert(entity, component)
    }

    /// 移除组件。
    pub fn remove<T: Component>(&mut self, entity: EcsEntity) -> Option<T> {
        if !self.contains(entity) {
            return None;
        }
        self.store_mut::<T>()?.remove(entity)
    }

    /// 只读取组件。
    pub fn get<T: Component>(&self, entity: EcsEntity) -> Option<&T> {
        if !self.contains(entity) {
            return None;
        }
        self.store::<T>()?.get(entity)
    }

    /// 可变取组件。
    pub fn get_mut<T: Component>(&mut self, entity: EcsEntity) -> Option<&mut T> {
        if !self.contains(entity) {
            return None;
        }
        self.store_mut::<T>()?.get_mut(entity)
    }

    /// 遍历某组件的全部实例（顺序为内部稠密数组顺序，**不保证**跨运行稳定；需要稳定序时按外部 `EntityId` 排序）。
    pub fn iter<T: Component>(&self) -> impl Iterator<Item = (EcsEntity, &T)> + '_ {
        self.store::<T>().into_iter().flat_map(ComponentStore::iter)
    }

    /// 取得空命令缓冲（不借用本世界；阶段末再 [`Self::apply`]）。
    pub fn commands(&mut self) -> EcsCommandBuffer {
        EcsCommandBuffer::new()
    }

    /// 按入队顺序应用结构变更。
    pub fn apply(&mut self, commands: EcsCommandBuffer) {
        for op in commands.into_ops() {
            op(self);
        }
    }

    fn ensure_store<T: Component>(&mut self) -> &mut ComponentStore<T> {
        let type_id = store_type_id::<T>();
        self.stores.entry(type_id).or_insert_with(|| Box::new(ComponentStore::<T>::default()));
        self.store_mut::<T>().expect("刚插入的组件存储")
    }

    fn store<T: Component>(&self) -> Option<&ComponentStore<T>> {
        self.stores.get(&store_type_id::<T>()).and_then(|s| s.as_any().downcast_ref())
    }

    fn store_mut<T: Component>(&mut self) -> Option<&mut ComponentStore<T>> {
        self.stores.get_mut(&store_type_id::<T>()).and_then(|s| s.as_any_mut().downcast_mut())
    }
}
