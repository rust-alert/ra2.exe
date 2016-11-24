//! 单位 / 载具 / 步兵等 techno 定义表。

use std::collections::BTreeMap;
use std::fmt;
use std::ops::Deref;

use serde::Deserialize;

use crate::id::{TypeId, WarheadId, WeaponId};

use super::ini_string::{deserialize_upper, parse_upper};
use super::{ArmorKind, HouseAllowList, PrerequisiteToken, ProductionCategory, TechnoCategory, WarheadName, WeaponName};


/// Techno 类型名（`DeploysInto=` 等类型引用）；空 = 未配置。
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
pub struct TechnoName {
    /// 规范化键（装载期大写）。
    pub name: String,
}

impl TechnoName {
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

impl Deref for TechnoName {
    type Target = str;

    fn deref(&self) -> &Self::Target {
        &self.name
    }
}

impl AsRef<str> for TechnoName {
    fn as_ref(&self) -> &str {
        &self.name
    }
}

impl std::borrow::Borrow<str> for TechnoName {
    fn borrow(&self) -> &str {
        &self.name
    }
}

impl fmt::Display for TechnoName {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.name)
    }
}

impl From<&str> for TechnoName {
    fn from(value: &str) -> Self {
        Self::parse(value)
    }
}

impl From<String> for TechnoName {
    fn from(value: String) -> Self {
        Self::parse(&value)
    }
}

impl PartialEq<str> for TechnoName {
    fn eq(&self, other: &str) -> bool {
        self.name.eq_ignore_ascii_case(other.trim())
    }
}

impl PartialEq<&str> for TechnoName {
    fn eq(&self, other: &&str) -> bool {
        self.name.eq_ignore_ascii_case(other.trim())
    }
}

impl PartialEq<TechnoName> for str {
    fn eq(&self, other: &TechnoName) -> bool {
        other.name.eq_ignore_ascii_case(self.trim())
    }
}

impl PartialEq<TechnoName> for &str {
    fn eq(&self, other: &TechnoName) -> bool {
        other.name.eq_ignore_ascii_case(self.trim())
    }
}

impl<'de> Deserialize<'de> for TechnoName {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        Ok(Self {
            name: deserialize_upper(deserializer)?,
        })
    }
}


/// 美术 `Image=` 资源名（缺省常等于类型 id）；空 = 未写。
#[derive(Debug, Clone, PartialEq, Eq, Hash, Default)]
pub struct ImageName {
    /// 规范化键（装载期大写）。
    pub name: String,
}

impl ImageName {
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

impl Deref for ImageName {
    type Target = str;

    fn deref(&self) -> &Self::Target {
        &self.name
    }
}

impl AsRef<str> for ImageName {
    fn as_ref(&self) -> &str {
        &self.name
    }
}

impl fmt::Display for ImageName {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.name)
    }
}

impl From<&str> for ImageName {
    fn from(value: &str) -> Self {
        Self::parse(value)
    }
}

impl From<String> for ImageName {
    fn from(value: String) -> Self {
        Self::parse(&value)
    }
}

impl PartialEq<str> for ImageName {
    fn eq(&self, other: &str) -> bool {
        self.name.eq_ignore_ascii_case(other.trim())
    }
}

impl PartialEq<&str> for ImageName {
    fn eq(&self, other: &&str) -> bool {
        self.name.eq_ignore_ascii_case(other.trim())
    }
}

impl PartialEq<ImageName> for str {
    fn eq(&self, other: &ImageName) -> bool {
        other.name.eq_ignore_ascii_case(self.trim())
    }
}

impl PartialEq<ImageName> for &str {
    fn eq(&self, other: &ImageName) -> bool {
        other.name.eq_ignore_ascii_case(self.trim())
    }
}

impl<'de> Deserialize<'de> for ImageName {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        Ok(Self {
            name: deserialize_upper(deserializer)?,
        })
    }
}

/// Techno 大类（与内容列表节对应）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum TechnoClass {
    /// 步兵。
    Infantry,
    /// 载具。
    Vehicle,
    /// 飞行器。
    Aircraft,
    /// 建筑（亦见于结构表）。
    Building,
}

impl TechnoClass {
    /// 对应生产类别（建筑本身不作为「被工厂生产」的默认类别）。
    pub fn production_category(self) -> Option<ProductionCategory> {
        match self {
            Self::Infantry => Some(ProductionCategory::Infantry),
            Self::Vehicle => Some(ProductionCategory::Vehicle),
            Self::Aircraft => Some(ProductionCategory::Aircraft),
            Self::Building => Some(ProductionCategory::Building),
        }
    }
}

/// 单条 techno 静态定义。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TechnoDefinition {
    /// 稳定类型编号。
    pub id: TypeId,
    /// 外部类型键。
    pub type_key: TechnoName,
    /// 大类。
    pub class: TechnoClass,
    /// 造价。
    pub cost: i32,
    /// 生命。
    pub strength: u32,
    /// 护甲种类（装载期由 `Armor=` 绑定）。
    pub armor: ArmorKind,
    /// 速度。
    pub speed: u32,
    /// `Owner=`：空名单 = 不限阵营。
    pub owner: HouseAllowList,
    /// `TechLevel`；`< 0` 表示不可建造。
    pub tech_level: i32,
    /// `Naval=yes`。
    pub naval: bool,
    /// `Agent=yes`（可渗透敌方建筑）。
    pub agent: bool,
    /// `Engineer=yes`（可占领敌方可俘建筑）。
    pub engineer: bool,
    /// `Harvester=yes`（采矿车）。
    pub harvester: bool,
    /// `Category=`（装载期一次解码）。
    pub category: TechnoCategory,
    /// 视野（格）；主武器 `Range=0` 时攻击射程回退用。
    pub sight: u32,
    /// 主武器名（`Primary`）；空表示未配置。
    pub primary: WeaponName,
    /// 主武器稳定 id；`WeaponId(0)` 表示未绑定。
    pub primary_id: WeaponId,
    /// 副武器名（`Secondary`）；空表示未配置。
    pub secondary: WeaponName,
    /// 副武器稳定 id；`WeaponId(0)` 表示未绑定。
    pub secondary_id: WeaponId,
    /// 主武器弹头名；空表示未配置（装载诊断 / 兼容；执行侧优先 `warhead_id`）。
    pub warhead: WarheadName,
    /// 主武器弹头稳定 id；`WarheadId(0)` 表示未绑定。
    pub warhead_id: WarheadId,
    /// `Prerequisite`：装载期绑定后的 token 列表；空 = 无前置。
    pub prerequisite: Vec<PrerequisiteToken>,
    /// `PrerequisiteOverride`：拥有任一即可绕过普通 Prerequisite。
    pub prerequisite_override: Vec<PrerequisiteToken>,
    /// `RequiredHouses=`：空名单 = 不限制；非空则 house 须命中其一。
    pub required_houses: HouseAllowList,
    /// `ForbiddenHouses=`：命中任一则不可造；空名单 = 不禁止。
    pub forbidden_houses: HouseAllowList,
    /// `BuildLimit`；`0` 表示不限。
    pub build_limit: i32,
    /// INI `BuildTime`（原版分钟档语义的整数）；`0` 表示缺省，生产侧回退默认 tick。
    pub build_time: u32,
    /// `RequiresStolenAlliedTech=yes`。
    pub requires_stolen_allied_tech: bool,
    /// `RequiresStolenSovietTech=yes`。
    pub requires_stolen_soviet_tech: bool,
    /// `RequiresStolenThirdTech=yes`。
    pub requires_stolen_third_tech: bool,
    /// INI `PixelSelectionBracketDelta`：选中血条竖直像素偏移（负值上移）。
    pub pixel_selection_bracket_delta: i32,
}

/// Techno 定义表。
#[derive(Debug, Clone, Default)]
pub struct TechnoDefinitions {
    by_key: BTreeMap<TechnoName, TechnoDefinition>,
}

impl TechnoDefinitions {
    /// 插入。
    pub fn insert(&mut self, def: TechnoDefinition) {
        self.by_key.insert(def.type_key.clone(), def);
    }

    /// 按键查找（大小写不敏感）。
    pub fn get(&self, type_key: &str) -> Option<&TechnoDefinition> {
        self.by_key.get(&TechnoName::parse(type_key))
    }

    /// 按已规范化的类型键查找。
    pub fn get_name(&self, type_key: &TechnoName) -> Option<&TechnoDefinition> {
        self.by_key.get(type_key)
    }

    /// 按稳定 id 查找。
    pub fn get_by_id(&self, id: TypeId) -> Option<&TechnoDefinition> {
        self.by_key.values().find(|t| t.id == id)
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
    pub fn iter(&self) -> impl Iterator<Item = &TechnoDefinition> {
        self.by_key.values()
    }

    /// 可变遍历（装载投影回填引用 id）。
    pub fn iter_mut(&mut self) -> impl Iterator<Item = &mut TechnoDefinition> {
        self.by_key.values_mut()
    }
}
