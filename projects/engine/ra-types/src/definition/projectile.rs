//! 抛射体定义表（由武器 `Projectile=` 引用）。

use std::collections::BTreeMap;
use std::fmt;
use std::ops::Deref;

use serde::Deserialize;

use crate::id::ProjectileId;

use super::ini_string::{deserialize_upper, parse_upper};


/// 抛射体节名（`Projectile=`）；空 = 未配置。
#[derive(Debug, Clone, PartialEq, Eq, Hash, Default)]
pub struct ProjectileName {
    /// 规范化键（装载期大写）。
    pub name: String,
}

impl ProjectileName {
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

impl Deref for ProjectileName {
    type Target = str;

    fn deref(&self) -> &Self::Target {
        &self.name
    }
}

impl AsRef<str> for ProjectileName {
    fn as_ref(&self) -> &str {
        &self.name
    }
}

impl fmt::Display for ProjectileName {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.name)
    }
}

impl From<&str> for ProjectileName {
    fn from(value: &str) -> Self {
        Self::parse(value)
    }
}

impl From<String> for ProjectileName {
    fn from(value: String) -> Self {
        Self::parse(&value)
    }
}

impl PartialEq<str> for ProjectileName {
    fn eq(&self, other: &str) -> bool {
        self.name.eq_ignore_ascii_case(other.trim())
    }
}

impl PartialEq<&str> for ProjectileName {
    fn eq(&self, other: &&str) -> bool {
        self.name.eq_ignore_ascii_case(other.trim())
    }
}

impl PartialEq<ProjectileName> for str {
    fn eq(&self, other: &ProjectileName) -> bool {
        other.name.eq_ignore_ascii_case(self.trim())
    }
}

impl PartialEq<ProjectileName> for &str {
    fn eq(&self, other: &ProjectileName) -> bool {
        other.name.eq_ignore_ascii_case(self.trim())
    }
}

impl<'de> Deserialize<'de> for ProjectileName {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        Ok(Self {
            name: deserialize_upper(deserializer)?,
        })
    }
}

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
