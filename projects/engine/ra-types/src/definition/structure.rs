//! 建筑与阵营定义表。

use std::collections::BTreeMap;
use std::fmt;

use serde::de::{self, Deserializer, Visitor};
use serde::Deserialize;

use crate::id::{HouseId, TypeId};

use super::{ArmorKind, BuiltinCapability, Foundation, HouseAllowList, ProductionProfile, StolenTechKind, SuperWeaponName};

/// 建造栏分类（INI `BuildCat=`）。
///
/// 侧栏 Q/W：非 `Combat` 进建筑页，`Combat` 进防御页。缺省视为建筑页。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
#[doc(hidden)]
pub enum BuildCat {
    /// 常规建筑（电厂 / 兵营 / 科技等；含缺省）。
    #[default]
    Building,
    /// 防御建筑与墙体。
    Combat,
}

impl BuildCat {
    /// 解析 INI `BuildCat=`；未知或空串 → [`Self::Building`]。
    pub fn parse(raw: &str) -> Self {
        match raw.trim() {
            s if s.eq_ignore_ascii_case("Combat") => Self::Combat,
            _ => Self::Building,
        }
    }

    /// 是否归入侧栏防御页（W）。
    pub fn is_defense_tab(self) -> bool {
        matches!(self, Self::Combat)
    }
}

impl<'de> Deserialize<'de> for BuildCat {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        struct BuildCatVisitor;

        impl<'de> Visitor<'de> for BuildCatVisitor {
            type Value = BuildCat;

            fn expecting(&self, f: &mut fmt::Formatter) -> fmt::Result {
                f.write_str("BuildCat= Building or Combat")
            }

            fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
            where
                E: de::Error,
            {
                Ok(BuildCat::parse(v))
            }

            fn visit_string<E>(self, v: String) -> Result<Self::Value, E>
            where
                E: de::Error,
            {
                Ok(BuildCat::parse(&v))
            }

            fn visit_none<E>(self) -> Result<Self::Value, E>
            where
                E: de::Error,
            {
                Ok(BuildCat::Building)
            }

            fn visit_unit<E>(self) -> Result<Self::Value, E>
            where
                E: de::Error,
            {
                Ok(BuildCat::Building)
            }
        }

        deserializer.deserialize_any(BuildCatVisitor)
    }
}

/// 建筑电力配置（正供电 / 耗电分离；是否需电）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[doc(hidden)]
pub struct PowerProfile {
    /// 供电量（≥0）。
    pub output: i32,
    /// 耗电量（≥0）。
    pub drain: i32,
    /// 是否需要电力才可运作 / 是否作为需电前置。
    pub requires_power: bool,
}

impl PowerProfile {
    /// 放置或拆除时对玩家电力表的增量：`output` 计入供电，`drain` 计入耗电。
    pub fn apply_to_player_delta(&self) -> (i32, i32) {
        (self.output, self.drain)
    }
}

/// 单条建筑静态定义（adaptor 冻结；引擎只读查询）。
#[derive(Debug, Clone, PartialEq, Eq)]
#[doc(hidden)]
pub struct StructureDefinition {
    /// 稳定类型编号。
    pub id: TypeId,
    /// 外部类型键（INI 节名，大写）。
    pub type_key: String,
    /// 电力。
    pub power: PowerProfile,
    /// 造价。
    pub cost: i32,
    /// 生命上限。
    pub strength: u32,
    /// 护甲种类（装载期由 `Armor=` 绑定）。
    pub armor: ArmorKind,
    /// 是否建造场。
    pub construction_yard: bool,
    /// 是否矿场。
    pub refinery: bool,
    /// INI `Radar=yes`（侧栏雷达开图）。
    pub radar: bool,
    /// INI `BuildCat=`（侧栏建筑 / 防御分页）。
    pub build_cat: BuildCat,
    /// `Capturable=yes`（可被工程师占领）。
    pub capturable: bool,
    /// 生产配置（若为工厂）。
    pub production: Option<ProductionProfile>,
    /// `Owner=`：空名单 = 不限阵营。
    pub owner: HouseAllowList,
    /// art / rules `Foundation=` 占地（原版主要在 art.ini）。
    pub foundation: Foundation,
    /// art / rules `Height`（缺省 2）：建筑选中框与 NW 血条竖向抬升。
    pub height: u16,
    /// INI `SuperWeapon=`：挂到该建筑的超级武器名（无则 `None`）。
    pub super_weapon: Option<SuperWeaponName>,
    /// 挂接超武的稳定 id；装载期绑定，执行侧优先于此。
    pub super_weapon_id: Option<TypeId>,
    /// 定义期能力声明。
    pub capabilities: Vec<BuiltinCapability>,
}

/// 单条阵营 / 房屋静态定义（由 rules `[Countries]` 投影）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HouseDefinition {
    /// 稳定房屋编号。
    pub id: HouseId,
    /// 外部房屋键（国家节名，大写）。
    pub type_key: String,
    /// `Side=` 原文（大写）；空表示未写。
    pub side: String,
    /// 由 `Side=` 推导的偷取科技类别；未知 Side 为 `None`。
    pub stolen_tech: Option<StolenTechKind>,
    /// 可出现在多人 / 遭遇战选用表。
    pub multiplay: bool,
}

/// 阵营 / 房屋定义表。
#[derive(Debug, Clone, Default)]
pub struct HouseDefinitions {
    by_key: BTreeMap<String, HouseDefinition>,
}

impl HouseDefinitions {
    /// 插入。
    pub fn insert(&mut self, def: HouseDefinition) {
        self.by_key.insert(def.type_key.clone(), def);
    }

    /// 按外部键查找（大小写不敏感）。
    pub fn get(&self, type_key: &str) -> Option<&HouseDefinition> {
        self.by_key.get(&type_key.to_ascii_uppercase())
    }

    /// 按稳定 id 查找。
    pub fn get_by_id(&self, id: HouseId) -> Option<&HouseDefinition> {
        self.by_key.values().find(|h| h.id == id)
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
    pub fn iter(&self) -> impl Iterator<Item = &HouseDefinition> {
        self.by_key.values()
    }
}

/// 建筑定义表（按外部 type_key 查询）。
#[derive(Debug, Clone, Default)]
#[doc(hidden)]
pub struct StructureDefinitions {
    by_key: BTreeMap<String, StructureDefinition>,
}

impl StructureDefinitions {
    /// 插入一条定义。
    pub fn insert(&mut self, def: StructureDefinition) {
        self.by_key.insert(def.type_key.clone(), def);
    }

    /// 按外部类型键查找（大小写不敏感）。
    pub fn get(&self, type_key: &str) -> Option<&StructureDefinition> {
        self.by_key.get(&type_key.to_ascii_uppercase())
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
    pub fn iter(&self) -> impl Iterator<Item = &StructureDefinition> {
        self.by_key.values()
    }
}
