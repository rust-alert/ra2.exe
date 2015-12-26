//! 单位 / 载具 / 步兵等 techno 定义表。

use std::collections::BTreeMap;

use crate::id::TypeId;

use super::ProductionCategory;

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
    pub type_key: String,
    /// 大类。
    pub class: TechnoClass,
    /// 造价。
    pub cost: i32,
    /// 生命。
    pub strength: u32,
    /// 护甲。
    pub armor: String,
    /// 速度。
    pub speed: u32,
    /// Owner 串。
    pub owner: String,
    /// `TechLevel`；`< 0` 表示不可建造。
    pub tech_level: i32,
    /// `Naval=yes`。
    pub naval: bool,
    /// `Agent=yes`（可渗透敌方建筑）。
    pub agent: bool,
    /// `Category`（如 `Soldier` / `Dog`）。
    pub category: String,
    /// 视野（格）；缺省攻击射程回退用。
    pub sight: u32,
    /// 主武器伤害；0 表示未配置。
    pub damage: u32,
    /// 主武器射程（格）；0 表示未配置。
    pub range: u32,
    /// 射速间隔（tick）；0 表示未配置。
    pub rof: u32,
    /// 主武器弹头键；空表示未配置。
    pub warhead: String,
    /// `Prerequisite` 逗号分隔 token（类型键或通用组名）；空 = 无前置。
    pub prerequisite: Vec<String>,
    /// `PrerequisiteOverride`：拥有任一即可绕过普通 Prerequisite。
    pub prerequisite_override: Vec<String>,
    /// `RequiredHouses`：非空时 house 必须命中其一。
    pub required_houses: Vec<String>,
    /// `ForbiddenHouses`：命中任一则不可造。
    pub forbidden_houses: Vec<String>,
    /// `BuildLimit`；`0` 表示不限。
    pub build_limit: i32,
    /// `RequiresStolenAlliedTech=yes`。
    pub requires_stolen_allied_tech: bool,
    /// `RequiresStolenSovietTech=yes`。
    pub requires_stolen_soviet_tech: bool,
    /// `RequiresStolenThirdTech=yes`。
    pub requires_stolen_third_tech: bool,
}

/// Techno 定义表。
#[derive(Debug, Clone, Default)]
pub struct TechnoDefinitions {
    by_key: BTreeMap<String, TechnoDefinition>,
}

impl TechnoDefinitions {
    /// 插入。
    pub fn insert(&mut self, def: TechnoDefinition) {
        self.by_key.insert(def.type_key.clone(), def);
    }

    /// 按键查找。
    pub fn get(&self, type_key: &str) -> Option<&TechnoDefinition> {
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

    /// 遍历。
    pub fn iter(&self) -> impl Iterator<Item = &TechnoDefinition> {
        self.by_key.values()
    }
}
