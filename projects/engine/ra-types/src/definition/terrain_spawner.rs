//! 地形矿柱产矿定义（装载期冻结）。

use std::collections::BTreeMap;

/// 可产矿的动画地形类型。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TerrainSpawnerDefinition {
    /// 外部类型键（如 `TIBTRE01`）。
    pub type_key: String,
    /// `AnimationProbability` × 1_000_000。
    pub animation_probability_micros: u32,
    /// `AnimationRate`（逻辑 tick / 动画帧），至少 1。
    pub animation_rate_ticks: u16,
}

/// 地形矿柱定义表。
#[derive(Debug, Clone, Default)]
pub struct TerrainSpawnerDefinitions {
    by_key: BTreeMap<String, TerrainSpawnerDefinition>,
}

impl TerrainSpawnerDefinitions {
    /// 插入。
    pub fn insert(&mut self, def: TerrainSpawnerDefinition) {
        self.by_key.insert(def.type_key.clone(), def);
    }

    /// 按键查找。
    pub fn get(&self, type_key: &str) -> Option<&TerrainSpawnerDefinition> {
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

    /// 迭代。
    pub fn iter(&self) -> impl Iterator<Item = &TerrainSpawnerDefinition> {
        self.by_key.values()
    }
}
