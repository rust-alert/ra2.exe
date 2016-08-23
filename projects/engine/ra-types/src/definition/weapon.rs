//! 武器定义表。

use std::collections::BTreeMap;

use crate::id::{WarheadId, WeaponId};

/// 单条武器静态定义（由 techno `Primary` / `Secondary` 引用）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WeaponDefinition {
    /// 稳定武器编号。
    pub id: WeaponId,
    /// 外部武器键（大写）。
    pub type_key: String,
    /// `Damage`。
    pub damage: u32,
    /// `Range`（格）。
    pub range: u32,
    /// `ROF`（tick）。
    pub rof: u32,
    /// 弹头键；空表示未配置。
    pub warhead: String,
    /// 弹头稳定 id；`WarheadId(0)` 表示未绑定。
    pub warhead_id: WarheadId,
}

/// 武器定义表。
#[derive(Debug, Clone, Default)]
pub struct WeaponDefinitions {
    by_key: BTreeMap<String, WeaponDefinition>,
}

impl WeaponDefinitions {
    /// 插入。
    pub fn insert(&mut self, def: WeaponDefinition) {
        self.by_key.insert(def.type_key.clone(), def);
    }

    /// 按键查找。
    pub fn get(&self, type_key: &str) -> Option<&WeaponDefinition> {
        self.by_key.get(&type_key.to_ascii_uppercase())
    }

    /// 按稳定 id 查找。
    pub fn get_by_id(&self, id: WeaponId) -> Option<&WeaponDefinition> {
        self.by_key.values().find(|w| w.id == id)
    }

    /// 条目数。
    pub fn len(&self) -> usize {
        self.by_key.len()
    }

    /// 是否空。
    pub fn is_empty(&self) -> bool {
        self.by_key.is_empty()
    }

    /// 遍历。
    pub fn iter(&self) -> impl Iterator<Item = &WeaponDefinition> {
        self.by_key.values()
    }

    /// 可变遍历（装载投影回填引用 id）。
    pub fn iter_mut(&mut self) -> impl Iterator<Item = &mut WeaponDefinition> {
        self.by_key.values_mut()
    }
}
