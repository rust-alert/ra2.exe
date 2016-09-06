//! 抛射体定义表（由武器 `Projectile=` 引用）。

use std::collections::BTreeMap;

use crate::id::ProjectileId;

/// 单条抛射体静态定义（装载期按名入库；字段可后续从节补齐）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProjectileDefinition {
    /// 稳定抛射体编号。
    pub id: ProjectileId,
    /// 外部抛射体键（大写）。
    pub type_key: String,
}

/// 抛射体定义表。
#[derive(Debug, Clone, Default)]
pub struct ProjectileDefinitions {
    by_key: BTreeMap<String, ProjectileDefinition>,
}

impl ProjectileDefinitions {
    /// 插入。
    pub fn insert(&mut self, def: ProjectileDefinition) {
        self.by_key.insert(def.type_key.clone(), def);
    }

    /// 按键查找。
    pub fn get(&self, type_key: &str) -> Option<&ProjectileDefinition> {
        self.by_key.get(&type_key.to_ascii_uppercase())
    }

    /// 按稳定 id 查找。
    pub fn get_by_id(&self, id: ProjectileId) -> Option<&ProjectileDefinition> {
        self.by_key.values().find(|p| p.id == id)
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
    pub fn iter(&self) -> impl Iterator<Item = &ProjectileDefinition> {
        self.by_key.values()
    }
}
