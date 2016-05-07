//! 建筑与阵营定义表。

use std::collections::BTreeMap;

use crate::id::TypeId;

use super::{BuiltinCapability, Foundation, ProductionProfile};

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
    /// `Capturable=yes`（可被工程师占领）。
    pub capturable: bool,
    /// 生产配置（若为工厂）。
    pub production: Option<ProductionProfile>,
    /// Owner 串（空表示不限）。
    pub owner: String,
    /// INI `Foundation=` 占地。
    pub foundation: Foundation,
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
