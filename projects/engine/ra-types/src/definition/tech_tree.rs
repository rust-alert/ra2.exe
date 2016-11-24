//! 科技树相关冻结定义（前置组、偷取科技与默认科技上限）。

use std::collections::BTreeMap;
use std::fmt;

use serde::de::{self, Deserializer, SeqAccess, Visitor};
use serde::Deserialize;

use crate::id::TypeId;

use super::{HouseName, TechnoName};

/// 渗透作战实验室后可获得的偷取科技类别（对齐 `RequiresStolen*Tech`）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum StolenTechKind {
    /// 盟军（`Side=GDI`）→ `RequiresStolenAlliedTech`。
    Allied,
    /// 苏军（`Side=Nod`）→ `RequiresStolenSovietTech`。
    Soviet,
    /// 第三势力（`Side=ThirdSide`）→ `RequiresStolenThirdTech`。
    Third,
}

impl StolenTechKind {
    /// 从国家 `Side=` 字符串映射；未知则 `None`。
    pub fn from_side(side: &str) -> Option<Self> {
        match side.trim().to_ascii_uppercase().as_str() {
            "GDI" => Some(Self::Allied),
            "NOD" => Some(Self::Soviet),
            "THIRDSIDE" | "THIRD" => Some(Self::Third),
            _ => None,
        }
    }
}

/// `[General]` 通用前置组名（`Prerequisite=` 中的 `POWER` / `FACTORY` 等）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PrerequisiteGroupKind {
    /// `POWER`
    Power,
    /// `FACTORY`
    Factory,
    /// `BARRACKS`
    Barracks,
    /// `RADAR`
    Radar,
    /// `TECH`
    Tech,
    /// `PROC`（含 alternate 列表）
    Proc,
}

impl PrerequisiteGroupKind {
    /// 解析通用组 token；非组名返回 `None`。
    pub fn parse(raw: &str) -> Option<Self> {
        match raw.trim().to_ascii_uppercase().as_str() {
            "POWER" => Some(Self::Power),
            "FACTORY" => Some(Self::Factory),
            "BARRACKS" => Some(Self::Barracks),
            "RADAR" => Some(Self::Radar),
            "TECH" => Some(Self::Tech),
            "PROC" => Some(Self::Proc),
            _ => None,
        }
    }
}

/// 装载期绑定后的前置 token（执行侧只认此枚举）。
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum PrerequisiteToken {
    /// 通用组：组内任一存活建筑即可。
    Group(PrerequisiteGroupKind),
    /// 已解析到 techno 表的类型引用。
    Type(TypeId),
    /// 未在 techno 表中找到的类型键；按存活建筑类型键比对。
    UnboundType(TechnoName),
}

impl PrerequisiteToken {
    /// 由 INI token 初解析：组名 → [`Self::Group`]，其余 → [`Self::UnboundType`]。
    pub fn parse_raw(raw: &str) -> Option<Self> {
        let upper = raw.trim().to_ascii_uppercase();
        if upper.is_empty() {
            return None;
        }
        if let Some(group) = PrerequisiteGroupKind::parse(&upper) {
            return Some(Self::Group(group));
        }
        Some(Self::UnboundType(TechnoName { name: upper }))
    }

    /// 将 [`Self::UnboundType`] 升级为 [`Self::Type`]（若类型表有该键）。
    pub fn bind_type_id(self, resolve: &impl Fn(&TechnoName) -> Option<TypeId>) -> Self {
        match self {
            Self::UnboundType(key) => resolve(&key).map(Self::Type).unwrap_or(Self::UnboundType(key)),
            other => other,
        }
    }
}

impl<'de> Deserialize<'de> for PrerequisiteToken {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        struct TokenVisitor;

        impl<'de> Visitor<'de> for TokenVisitor {
            type Value = PrerequisiteToken;

            fn expecting(&self, f: &mut fmt::Formatter) -> fmt::Result {
                f.write_str("prerequisite token")
            }

            fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
            where
                E: de::Error,
            {
                PrerequisiteToken::parse_raw(v).ok_or_else(|| E::custom("empty prerequisite token"))
            }

            fn visit_string<E>(self, v: String) -> Result<Self::Value, E>
            where
                E: de::Error,
            {
                PrerequisiteToken::parse_raw(&v).ok_or_else(|| E::custom("empty prerequisite token"))
            }
        }

        deserializer.deserialize_any(TokenVisitor)
    }
}

/// `Prerequisite=` / `PrerequisiteOverride=` 列表（装载期一次解码）。
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct PrerequisiteList {
    tokens: Vec<PrerequisiteToken>,
}

impl PrerequisiteList {
    /// 空列表。
    pub fn empty() -> Self {
        Self { tokens: Vec::new() }
    }

    /// 由 token 构造。
    pub fn from_tokens(tokens: impl IntoIterator<Item = PrerequisiteToken>) -> Self {
        Self {
            tokens: tokens.into_iter().collect(),
        }
    }

    /// 是否为空。
    pub fn is_empty(&self) -> bool {
        self.tokens.is_empty()
    }

    /// 长度。
    pub fn len(&self) -> usize {
        self.tokens.len()
    }

    /// 迭代。
    pub fn iter(&self) -> impl Iterator<Item = &PrerequisiteToken> {
        self.tokens.iter()
    }

    /// 取出内部列表（投影 / 绑定用）。
    pub fn into_vec(self) -> Vec<PrerequisiteToken> {
        self.tokens
    }

    /// 绑定类型引用后返回新列表。
    pub fn bind_type_ids(self, resolve: &impl Fn(&TechnoName) -> Option<TypeId>) -> Self {
        Self {
            tokens: self.tokens.into_iter().map(|t| t.bind_type_id(resolve)).collect(),
        }
    }
}

impl From<PrerequisiteList> for Vec<PrerequisiteToken> {
    fn from(value: PrerequisiteList) -> Self {
        value.tokens
    }
}

impl<'de> Deserialize<'de> for PrerequisiteList {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        struct ListVisitor;

        impl<'de> Visitor<'de> for ListVisitor {
            type Value = PrerequisiteList;

            fn expecting(&self, f: &mut fmt::Formatter) -> fmt::Result {
                f.write_str("comma-separated prerequisite tokens")
            }

            fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
            where
                E: de::Error,
            {
                Ok(PrerequisiteList::from_tokens(
                    v.split(|c| c == ',' || c == ';' || c == '|')
                        .filter_map(PrerequisiteToken::parse_raw),
                ))
            }

            fn visit_string<E>(self, v: String) -> Result<Self::Value, E>
            where
                E: de::Error,
            {
                self.visit_str(&v)
            }

            fn visit_seq<A>(self, mut seq: A) -> Result<Self::Value, A::Error>
            where
                A: SeqAccess<'de>,
            {
                let mut tokens = Vec::new();
                while let Some(part) = seq.next_element::<String>()? {
                    if let Some(token) = PrerequisiteToken::parse_raw(&part) {
                        tokens.push(token);
                    }
                }
                Ok(PrerequisiteList::from_tokens(tokens))
            }

            fn visit_none<E>(self) -> Result<Self::Value, E>
            where
                E: de::Error,
            {
                Ok(PrerequisiteList::empty())
            }

            fn visit_unit<E>(self) -> Result<Self::Value, E>
            where
                E: de::Error,
            {
                Ok(PrerequisiteList::empty())
            }
        }

        deserializer.deserialize_any(ListVisitor)
    }
}

/// `[General]` 通用前置组：组内任一存活建筑即可满足对应 token。
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct PrerequisiteGroups {
    /// `PrerequisitePower` → token `POWER`。
    pub power: Vec<TechnoName>,
    /// `PrerequisiteFactory` → token `FACTORY`。
    pub factory: Vec<TechnoName>,
    /// `PrerequisiteBarracks` → token `BARRACKS`。
    pub barracks: Vec<TechnoName>,
    /// `PrerequisiteRadar` → token `RADAR`。
    pub radar: Vec<TechnoName>,
    /// `PrerequisiteTech` → token `TECH`。
    pub tech: Vec<TechnoName>,
    /// `PrerequisiteProc` → token `PROC`。
    pub proc: Vec<TechnoName>,
    /// `PrerequisiteProcAlternate`（并入 `PROC` 判定）。
    pub proc_alternate: Vec<TechnoName>,
}

impl PrerequisiteGroups {
    /// 按通用 token 名取类型键列表（大小写不敏感）。未知 token 返回空切片。
    pub fn types_for_token(&self, token: &str) -> &[TechnoName] {
        match PrerequisiteGroupKind::parse(token) {
            Some(kind) => self.types_for_kind(kind),
            None => &[],
        }
    }

    /// 按组枚举取类型键列表（`Proc` 仅主列表，完整判定用 [`Self::proc_all`]）。
    pub fn types_for_kind(&self, kind: PrerequisiteGroupKind) -> &[TechnoName] {
        match kind {
            PrerequisiteGroupKind::Power => self.power.as_slice(),
            PrerequisiteGroupKind::Factory => self.factory.as_slice(),
            PrerequisiteGroupKind::Barracks => self.barracks.as_slice(),
            PrerequisiteGroupKind::Radar => self.radar.as_slice(),
            PrerequisiteGroupKind::Tech => self.tech.as_slice(),
            PrerequisiteGroupKind::Proc => self.proc.as_slice(),
        }
    }

    /// `PROC` 判定用的全部类型键（主列表 + alternate）。
    pub fn proc_all(&self) -> impl Iterator<Item = &str> {
        self.proc.iter().chain(self.proc_alternate.iter()).map(TechnoName::as_str)
    }

    /// 类型键是否属于 `TECH` 通用组（作战实验室等）。
    pub fn is_tech_building(&self, type_key: &str) -> bool {
        let want = TechnoName::parse(type_key);
        self.tech.iter().any(|t| t == &want)
    }
}

/// house id → 渗透其科技建筑时授予的偷取科技类别。
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct HouseStolenTechMap {
    by_house: BTreeMap<HouseName, StolenTechKind>,
}

impl HouseStolenTechMap {
    /// 插入一条映射。
    pub fn insert(&mut self, house: HouseName, kind: StolenTechKind) {
        self.by_house.insert(house, kind);
    }

    /// 按 house 查找（大小写不敏感）。
    pub fn get(&self, house: &str) -> Option<StolenTechKind> {
        self.by_house.get(&HouseName::parse(house)).copied()
    }

    /// 按已规范化的 house 键查找。
    pub fn get_name(&self, house: &HouseName) -> Option<StolenTechKind> {
        self.by_house.get(house).copied()
    }

    /// 条目数。
    pub fn len(&self) -> usize {
        self.by_house.len()
    }

    /// 是否空。
    pub fn is_empty(&self) -> bool {
        self.by_house.is_empty()
    }
}
