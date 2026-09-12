//! 超级武器定义表（`[SuperWeaponTypes]`）。

use std::collections::BTreeMap;
use std::fmt;
use std::ops::Deref;

use serde::Deserialize;

use crate::id::{TypeId, WeaponId};

use super::ini_string::{deserialize_upper, parse_upper};
use super::{ImageName, UiName, WeaponName};


/// 超级武器类型名（`SuperWeapon=`）；空 = 未配置。
#[derive(Debug, Clone, PartialEq, Eq, Hash, Default)]
pub struct SuperWeaponName {
    /// 规范化键（装载期大写）。
    pub name: String,
}

impl SuperWeaponName {
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

impl Deref for SuperWeaponName {
    type Target = str;

    fn deref(&self) -> &Self::Target {
        &self.name
    }
}

impl AsRef<str> for SuperWeaponName {
    fn as_ref(&self) -> &str {
        &self.name
    }
}

impl fmt::Display for SuperWeaponName {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.name)
    }
}

impl From<&str> for SuperWeaponName {
    fn from(value: &str) -> Self {
        Self::parse(value)
    }
}

impl From<String> for SuperWeaponName {
    fn from(value: String) -> Self {
        Self::parse(&value)
    }
}

impl PartialEq<str> for SuperWeaponName {
    fn eq(&self, other: &str) -> bool {
        self.name.eq_ignore_ascii_case(other.trim())
    }
}

impl PartialEq<&str> for SuperWeaponName {
    fn eq(&self, other: &&str) -> bool {
        self.name.eq_ignore_ascii_case(other.trim())
    }
}

impl PartialEq<SuperWeaponName> for str {
    fn eq(&self, other: &SuperWeaponName) -> bool {
        other.name.eq_ignore_ascii_case(self.trim())
    }
}

impl PartialEq<SuperWeaponName> for &str {
    fn eq(&self, other: &SuperWeaponName) -> bool {
        other.name.eq_ignore_ascii_case(self.trim())
    }
}

impl<'de> Deserialize<'de> for SuperWeaponName {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        Ok(Self {
            name: deserialize_upper(deserializer)?,
        })
    }
}


/// 超武 `Type=` 玩法类型名；空 = 未写。
#[derive(Debug, Clone, PartialEq, Eq, Hash, Default)]
pub struct SuperWeaponKindName {
    /// 规范化键（装载期大写）。
    pub name: String,
}

impl SuperWeaponKindName {
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

impl Deref for SuperWeaponKindName {
    type Target = str;

    fn deref(&self) -> &Self::Target {
        &self.name
    }
}

impl AsRef<str> for SuperWeaponKindName {
    fn as_ref(&self) -> &str {
        &self.name
    }
}

impl fmt::Display for SuperWeaponKindName {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.name)
    }
}

impl From<&str> for SuperWeaponKindName {
    fn from(value: &str) -> Self {
        Self::parse(value)
    }
}

impl From<String> for SuperWeaponKindName {
    fn from(value: String) -> Self {
        Self::parse(&value)
    }
}

impl PartialEq<str> for SuperWeaponKindName {
    fn eq(&self, other: &str) -> bool {
        self.name.eq_ignore_ascii_case(other.trim())
    }
}

impl PartialEq<&str> for SuperWeaponKindName {
    fn eq(&self, other: &&str) -> bool {
        self.name.eq_ignore_ascii_case(other.trim())
    }
}

impl PartialEq<SuperWeaponKindName> for str {
    fn eq(&self, other: &SuperWeaponKindName) -> bool {
        other.name.eq_ignore_ascii_case(self.trim())
    }
}

impl PartialEq<SuperWeaponKindName> for &str {
    fn eq(&self, other: &SuperWeaponKindName) -> bool {
        other.name.eq_ignore_ascii_case(self.trim())
    }
}

impl<'de> Deserialize<'de> for SuperWeaponKindName {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        Ok(Self {
            name: deserialize_upper(deserializer)?,
        })
    }
}


/// 超武 `Action=` 动作名；空 = 未写。
#[derive(Debug, Clone, PartialEq, Eq, Hash, Default)]
pub struct SuperWeaponActionName {
    /// 规范化键（装载期大写）。
    pub name: String,
}

impl SuperWeaponActionName {
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

impl Deref for SuperWeaponActionName {
    type Target = str;

    fn deref(&self) -> &Self::Target {
        &self.name
    }
}

impl AsRef<str> for SuperWeaponActionName {
    fn as_ref(&self) -> &str {
        &self.name
    }
}

impl fmt::Display for SuperWeaponActionName {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.name)
    }
}

impl From<&str> for SuperWeaponActionName {
    fn from(value: &str) -> Self {
        Self::parse(value)
    }
}

impl From<String> for SuperWeaponActionName {
    fn from(value: String) -> Self {
        Self::parse(&value)
    }
}

impl PartialEq<str> for SuperWeaponActionName {
    fn eq(&self, other: &str) -> bool {
        self.name.eq_ignore_ascii_case(other.trim())
    }
}

impl PartialEq<&str> for SuperWeaponActionName {
    fn eq(&self, other: &&str) -> bool {
        self.name.eq_ignore_ascii_case(other.trim())
    }
}

impl PartialEq<SuperWeaponActionName> for str {
    fn eq(&self, other: &SuperWeaponActionName) -> bool {
        other.name.eq_ignore_ascii_case(self.trim())
    }
}

impl PartialEq<SuperWeaponActionName> for &str {
    fn eq(&self, other: &SuperWeaponActionName) -> bool {
        other.name.eq_ignore_ascii_case(self.trim())
    }
}

impl<'de> Deserialize<'de> for SuperWeaponActionName {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        Ok(Self {
            name: deserialize_upper(deserializer)?,
        })
    }
}

/// 单条超级武器静态定义（adaptor 冻结；引擎只读查询）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SuperWeaponDefinition {
    /// 稳定类型编号。
    pub id: TypeId,
    /// 外部类型键（INI 节名，装载期一次解码为大写）。
    pub type_key: SuperWeaponName,
    /// `UIName=` CSF 键（装载期一次解码）；空表示未写。
    pub ui_name: UiName,
    /// `Type=` 玩法类型名（可空）。
    pub kind: SuperWeaponKindName,
    /// `Action=` 动作名（可空）。
    pub action: SuperWeaponActionName,
    /// `RechargeTime=` 原版充能档（整数；`0` 表示缺省/未写）。
    pub recharge_time: i32,
    /// `SidebarImage=`（装载期一次解码）；空表示未写。
    pub sidebar_image: ImageName,
    /// `Weapon=` 关联武器名（可空，诊断用）。
    pub weapon: WeaponName,
    /// `Weapon=` 绑定到武器表的稳定 id；`WeaponId(0)` 表示未配置或未命中。
    pub weapon_id: WeaponId,
}

/// 超级武器定义表（按外部 type_key 查询）。
#[derive(Debug, Clone, Default)]
pub struct SuperWeaponDefinitions {
    by_key: BTreeMap<String, SuperWeaponDefinition>,
}

impl SuperWeaponDefinitions {
    /// 插入一条定义。
    pub fn insert(&mut self, def: SuperWeaponDefinition) {
        self.by_key.insert(def.type_key.as_str().to_string(), def);
    }

    /// 按外部类型键查找（大小写不敏感）。
    pub fn get(&self, type_key: &str) -> Option<&SuperWeaponDefinition> {
        self.by_key.get(&type_key.to_ascii_uppercase())
    }

    /// 按稳定 id 查找。
    pub fn get_by_id(&self, id: TypeId) -> Option<&SuperWeaponDefinition> {
        self.by_key.values().find(|d| d.id == id)
    }

    /// 条目数。
    pub fn len(&self) -> usize {
        self.by_key.len()
    }

    /// 是否空表。
    pub fn is_empty(&self) -> bool {
        self.by_key.is_empty()
    }

    /// 遍历。
    pub fn iter(&self) -> impl Iterator<Item = &SuperWeaponDefinition> {
        self.by_key.values()
    }

    /// 可变遍历（装载投影回填引用 id）。
    pub fn iter_mut(&mut self) -> impl Iterator<Item = &mut SuperWeaponDefinition> {
        self.by_key.values_mut()
    }
}
