//! 弹头定义表。

use std::collections::BTreeMap;

use crate::id::TypeId;

/// 护甲名在 `Verses` 列表中的固定顺序（11 项）。
pub const ARMOR_ORDER: [&str; 11] =
    ["none", "flak", "plate", "light", "medium", "heavy", "wood", "steel", "concrete", "special_1", "special_2"];

/// 护甲名 → `Verses` 下标；未知护甲按 `none`（0）。
pub fn armor_index(armor: &str) -> usize {
    let key = armor.trim().to_ascii_lowercase();
    ARMOR_ORDER.iter().position(|a| *a == key).unwrap_or(0)
}

/// 单条弹头定义（对各护甲的伤害百分比）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WarheadDefinition {
    /// 稳定类型编号。
    pub id: TypeId,
    /// 外部弹头键。
    pub type_key: String,
    /// 对应 [`ARMOR_ORDER`] 的百分比倍率。
    pub verses: [u32; 11],
}

/// 弹头定义表。
#[derive(Debug, Clone, Default)]
pub struct WarheadDefinitions {
    by_key: BTreeMap<String, WarheadDefinition>,
}

impl WarheadDefinitions {
    /// 插入。
    pub fn insert(&mut self, def: WarheadDefinition) {
        self.by_key.insert(def.type_key.clone(), def);
    }

    /// 按键查找。
    pub fn get(&self, type_key: &str) -> Option<&WarheadDefinition> {
        self.by_key.get(&type_key.to_ascii_uppercase())
    }

    /// 条目数。
    pub fn len(&self) -> usize {
        self.by_key.len()
    }

    /// 是否空。
    pub fn is_empty(&self) -> bool {
        self.by_key.is_empty()
    }

    /// 迭代。
    pub fn iter(&self) -> impl Iterator<Item = &WarheadDefinition> {
        self.by_key.values()
    }
}
