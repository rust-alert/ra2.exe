//! 弹头定义表。

use std::collections::BTreeMap;

use crate::id::WarheadId;

use super::WarheadVerses;

/// 单条弹头定义（对各护甲的伤害百分比）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WarheadDefinition {
    /// 稳定弹头编号。
    pub id: WarheadId,
    /// 外部弹头键。
    pub type_key: String,
    /// 对应 [`super::ARMOR_ORDER`] 的百分比倍率。
    pub verses: WarheadVerses,
    /// `Spread=` 溅射半径（格）；缺省 0。
    pub spread: u32,
    /// `ProneDamage=` 对卧倒单位的伤害百分比；缺省 100。
    pub prone_damage: u32,
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

    /// 按稳定 id 查找。
    pub fn get_by_id(&self, id: WarheadId) -> Option<&WarheadDefinition> {
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

    /// 迭代。
    pub fn iter(&self) -> impl Iterator<Item = &WarheadDefinition> {
        self.by_key.values()
    }
}
