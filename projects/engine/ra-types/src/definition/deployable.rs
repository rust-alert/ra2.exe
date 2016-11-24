//! 部署关系定义（如 MCV → 建造场）。

use std::collections::BTreeMap;

use crate::id::TypeId;

use super::TechnoName;

/// 部署放置策略（骨架）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum DeploymentPlacement {
    /// 原地变为目标类型。
    #[default]
    InPlace,
}

/// 可部署定义：源类型 → 目标类型。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DeployableDefinition {
    /// 源类型编号。
    pub source: TypeId,
    /// 源外部键。
    pub source_key: TechnoName,
    /// 目标类型编号。
    pub target: TypeId,
    /// 目标外部键。
    pub target_key: TechnoName,
    /// 放置策略。
    pub placement: DeploymentPlacement,
}

/// 部署关系表（按源 type_key）。
#[derive(Debug, Clone, Default)]
pub struct DeployableDefinitions {
    by_source: BTreeMap<TechnoName, DeployableDefinition>,
}

impl DeployableDefinitions {
    /// 插入。
    pub fn insert(&mut self, def: DeployableDefinition) {
        self.by_source.insert(def.source_key.clone(), def);
    }

    /// 按源类型键查找（大小写不敏感）。
    pub fn get(&self, source_key: &str) -> Option<&DeployableDefinition> {
        self.by_source.get(&TechnoName::parse(source_key))
    }

    /// 按已规范化的源类型键查找。
    pub fn get_name(&self, source_key: &TechnoName) -> Option<&DeployableDefinition> {
        self.by_source.get(source_key)
    }

    /// 条目数。
    pub fn len(&self) -> usize {
        self.by_source.len()
    }

    /// 是否空。
    pub fn is_empty(&self) -> bool {
        self.by_source.is_empty()
    }

    /// 遍历。
    pub fn iter(&self) -> impl Iterator<Item = &DeployableDefinition> {
        self.by_source.values()
    }
}
