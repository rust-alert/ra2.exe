//! 稀疏集合组件存储。

use std::any::{Any, TypeId};

use crate::{Component, entity::EcsEntity};

/// 某组件类型的稀疏集合：`sparse[slot] → dense 下标`。
#[derive(Debug, Clone)]
pub(crate) struct ComponentStore<T> {
    dense: Vec<T>,
    entities: Vec<EcsEntity>,
    /// `slot → dense index`；空槽为 `None`。
    sparse: Vec<Option<u32>>,
}

impl<T> Default for ComponentStore<T> {
    fn default() -> Self {
        Self { dense: Vec::new(), entities: Vec::new(), sparse: Vec::new() }
    }
}

impl<T> ComponentStore<T> {
    fn ensure_sparse(&mut self, slot: u32) {
        let need = slot as usize + 1;
        if self.sparse.len() < need {
            self.sparse.resize(need, None);
        }
    }

    pub(crate) fn insert(&mut self, entity: EcsEntity, value: T) -> Option<T> {
        self.ensure_sparse(entity.slot);
        if let Some(idx) = self.sparse[entity.slot as usize] {
            let old = std::mem::replace(&mut self.dense[idx as usize], value);
            self.entities[idx as usize] = entity;
            Some(old)
        }
        else {
            let idx = self.dense.len() as u32;
            self.dense.push(value);
            self.entities.push(entity);
            self.sparse[entity.slot as usize] = Some(idx);
            None
        }
    }

    pub(crate) fn remove(&mut self, entity: EcsEntity) -> Option<T> {
        let slot = entity.slot as usize;
        let idx = self.sparse.get(slot).copied().flatten()?;
        let idx = idx as usize;
        if self.entities.get(idx).copied() != Some(entity) {
            return None;
        }

        let removed = self.dense.swap_remove(idx);
        self.entities.swap_remove(idx);
        self.sparse[slot] = None;

        if idx < self.dense.len() {
            let moved = self.entities[idx];
            self.ensure_sparse(moved.slot);
            self.sparse[moved.slot as usize] = Some(idx as u32);
        }
        Some(removed)
    }

    pub(crate) fn get(&self, entity: EcsEntity) -> Option<&T> {
        let idx = self.sparse.get(entity.slot as usize).copied().flatten()?;
        if self.entities.get(idx as usize).copied() != Some(entity) {
            return None;
        }
        self.dense.get(idx as usize)
    }

    pub(crate) fn get_mut(&mut self, entity: EcsEntity) -> Option<&mut T> {
        let idx = self.sparse.get(entity.slot as usize).copied().flatten()?;
        if self.entities.get(idx as usize).copied() != Some(entity) {
            return None;
        }
        self.dense.get_mut(idx as usize)
    }

    pub(crate) fn iter(&self) -> impl Iterator<Item = (EcsEntity, &T)> {
        self.entities.iter().copied().zip(self.dense.iter())
    }

    pub(crate) fn clear_slot(&mut self, slot: u32) {
        let Some(idx) = self.sparse.get(slot as usize).copied().flatten()
        else {
            return;
        };
        let idx = idx as usize;
        let entity = self.entities[idx];
        if entity.slot != slot {
            return;
        }
        self.dense.swap_remove(idx);
        self.entities.swap_remove(idx);
        self.sparse[slot as usize] = None;
        if idx < self.dense.len() {
            let moved = self.entities[idx];
            self.ensure_sparse(moved.slot);
            self.sparse[moved.slot as usize] = Some(idx as u32);
        }
    }
}

pub(crate) trait ErasedStore: Any + Send {
    fn as_any(&self) -> &dyn Any;
    fn as_any_mut(&mut self) -> &mut dyn Any;
    fn clear_slot(&mut self, slot: u32);
    fn clone_box(&self) -> Box<dyn ErasedStore>;
}

impl<T: Component> ErasedStore for ComponentStore<T> {
    fn as_any(&self) -> &dyn Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }

    fn clear_slot(&mut self, slot: u32) {
        ComponentStore::clear_slot(self, slot);
    }

    fn clone_box(&self) -> Box<dyn ErasedStore> {
        Box::new(self.clone())
    }
}

pub(crate) fn store_type_id<T: Component>() -> TypeId {
    TypeId::of::<ComponentStore<T>>()
}
