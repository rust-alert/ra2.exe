//! 武器定义表。

use std::collections::BTreeMap;
use std::fmt;
use std::ops::Deref;

use serde::Deserialize;

use crate::id::{ProjectileId, WarheadId, WeaponId};

use super::ini_string::{deserialize_upper, parse_upper};
use super::{ProjectileName, WarheadName};

/// 武器节名（`Primary` / `Secondary` / `Weapon=` 等）；空 = 未配置。
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
pub struct WeaponName {
    /// 规范化键（装载期大写）。
    pub name: String,
}

impl WeaponName {
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

impl Deref for WeaponName {
    type Target = str;

    fn deref(&self) -> &Self::Target {
        &self.name
    }
}

impl AsRef<str> for WeaponName {
    fn as_ref(&self) -> &str {
        &self.name
    }
}

impl std::borrow::Borrow<str> for WeaponName {
    fn borrow(&self) -> &str {
        &self.name
    }
}

impl fmt::Display for WeaponName {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.name)
    }
}

impl From<&str> for WeaponName {
    fn from(value: &str) -> Self {
        Self::parse(value)
    }
}

impl From<String> for WeaponName {
    fn from(value: String) -> Self {
        Self::parse(&value)
    }
}

impl PartialEq<str> for WeaponName {
    fn eq(&self, other: &str) -> bool {
        self.name.eq_ignore_ascii_case(other.trim())
    }
}

impl PartialEq<&str> for WeaponName {
    fn eq(&self, other: &&str) -> bool {
        self.name.eq_ignore_ascii_case(other.trim())
    }
}

impl PartialEq<WeaponName> for str {
    fn eq(&self, other: &WeaponName) -> bool {
        other.name.eq_ignore_ascii_case(self.trim())
    }
}

impl PartialEq<WeaponName> for &str {
    fn eq(&self, other: &WeaponName) -> bool {
        other.name.eq_ignore_ascii_case(self.trim())
    }
}

impl<'de> Deserialize<'de> for WeaponName {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        Ok(Self {
            name: deserialize_upper(deserializer)?,
        })
    }
}

/// 单条武器静态定义（由 techno `Primary` / 超武 `Weapon=` 等引用）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WeaponDefinition {
    /// 稳定武器编号。
    pub id: WeaponId,
    /// 外部武器键。
    pub type_key: WeaponName,
    /// `Damage`。
    pub damage: u32,
    /// `Range`（格）。
    pub range: u32,
    /// `ROF`（tick）。
    pub rof: u32,
    /// 弹头名；空表示未配置。
    pub warhead: WarheadName,
    /// 弹头稳定 id；`WarheadId(0)` 表示未绑定。
    pub warhead_id: WarheadId,
    /// 抛射体名（`Projectile=`）；空表示未配置。
    pub projectile: ProjectileName,
    /// 抛射体稳定 id；`ProjectileId(0)` 表示未绑定。
    pub projectile_id: ProjectileId,
}

/// 武器定义表。
#[derive(Debug, Clone, Default)]
pub struct WeaponDefinitions {
    by_key: BTreeMap<WeaponName, WeaponDefinition>,
}

impl WeaponDefinitions {
    /// 插入。
    pub fn insert(&mut self, def: WeaponDefinition) {
        self.by_key.insert(def.type_key.clone(), def);
    }

    /// 按键查找（大小写不敏感）。
    pub fn get(&self, type_key: &str) -> Option<&WeaponDefinition> {
        self.by_key.get(&WeaponName::parse(type_key))
    }

    /// 按已规范化的武器键查找。
    pub fn get_name(&self, type_key: &WeaponName) -> Option<&WeaponDefinition> {
        self.by_key.get(type_key)
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
