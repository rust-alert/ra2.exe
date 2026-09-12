//! 弹头定义表。

use std::collections::BTreeMap;
use std::fmt;
use std::ops::Deref;

use serde::Deserialize;

use crate::id::WarheadId;

use super::WarheadVerses;
use super::ini_string::{deserialize_upper, parse_upper};


/// 弹头节名（`Warhead=`）；空 = 未配置。
#[derive(Debug, Clone, PartialEq, Eq, Hash, Default)]
pub struct WarheadName {
    /// 规范化键（装载期大写）。
    pub name: String,
}

impl WarheadName {
    /// 修剪并规范为大写；空串表示未配置。
    pub fn parse(raw: &str) -> Self {
        Self { name: parse_upper(raw) }
    }

    /// 底层键文本。
    pub fn as_str(&self) -> &str {
        &self.name
    }

    /// 是否未配置。
    pub fn is_empty(&self) -> bool {
        self.name.is_empty()
    }
}

impl Deref for WarheadName {
    type Target = str;

    fn deref(&self) -> &Self::Target {
        &self.name
    }
}

impl AsRef<str> for WarheadName {
    fn as_ref(&self) -> &str {
        &self.name
    }
}

impl fmt::Display for WarheadName {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.name)
    }
}

impl From<&str> for WarheadName {
    fn from(value: &str) -> Self {
        Self::parse(value)
    }
}

impl From<String> for WarheadName {
    fn from(value: String) -> Self {
        Self::parse(&value)
    }
}

impl PartialEq<str> for WarheadName {
    fn eq(&self, other: &str) -> bool {
        self.name.eq_ignore_ascii_case(other.trim())
    }
}

impl PartialEq<&str> for WarheadName {
    fn eq(&self, other: &&str) -> bool {
        self.name.eq_ignore_ascii_case(other.trim())
    }
}

impl PartialEq<WarheadName> for str {
    fn eq(&self, other: &WarheadName) -> bool {
        other.name.eq_ignore_ascii_case(self.trim())
    }
}

impl PartialEq<WarheadName> for &str {
    fn eq(&self, other: &WarheadName) -> bool {
        other.name.eq_ignore_ascii_case(self.trim())
    }
}

impl<'de> Deserialize<'de> for WarheadName {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        Ok(Self {
            name: deserialize_upper(deserializer)?,
        })
    }
}

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
