//! 建筑与阵营定义表。

use std::collections::BTreeMap;

use crate::id::TypeId;

use super::{BuiltinCapability, Foundation, ProductionProfile};

/// 建造栏分类（INI `BuildCat=`）。
///
/// 侧栏 Q/W：非 `Combat` 进建筑页，`Combat` 进防御页。缺省视为建筑页。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
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

/// 建筑电力配置（正供电 / 耗电分离；是否需电）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
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
    /// 护甲名。
    pub armor: String,
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
    /// Owner 串（空表示不限）。
    pub owner: String,
    /// INI `Foundation=` 占地。
    pub foundation: Foundation,
    /// INI `SuperWeapon=`：挂到该建筑的超级武器类型键（大写；无则 `None`）。
    pub super_weapon: Option<String>,
    /// 定义期能力声明。
    pub capabilities: Vec<BuiltinCapability>,
}

/// 阵营 / 房屋定义集合（骨架）。
#[derive(Debug, Clone, Default)]
pub struct HouseDefinitions {
    /// 条目数占位。
    pub count: u32,
}

/// 建筑定义表（按外部 type_key 查询）。
#[derive(Debug, Clone, Default)]
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

#[cfg(test)]
mod tests {
    use super::BuildCat;

    #[test]
    fn build_cat_combat_is_defense_tab() {
        assert_eq!(BuildCat::parse("Combat"), BuildCat::Combat);
        assert_eq!(BuildCat::parse("combat"), BuildCat::Combat);
        assert!(BuildCat::Combat.is_defense_tab());
    }

    #[test]
    fn build_cat_missing_or_other_goes_to_building_tab() {
        assert_eq!(BuildCat::parse(""), BuildCat::Building);
        assert_eq!(BuildCat::parse("Tech"), BuildCat::Building);
        assert_eq!(BuildCat::parse("Power"), BuildCat::Building);
        assert_eq!(BuildCat::parse("Resource"), BuildCat::Building);
        assert!(!BuildCat::Building.is_defense_tab());
    }
}
