//! 生产类别与工厂生产配置。

/// 工厂可生产的单位大类（由 INI `Factory=` 等解释而来）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ProductionCategory {
    /// 步兵。
    Infantry,
    /// 载具。
    Vehicle,
    /// 飞行器。
    Aircraft,
    /// 建筑（建造栏）。
    Building,
}

/// 生产设施配置。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProductionProfile {
    /// 该工厂产出的类别。
    pub category: ProductionCategory,
}

/// 生产 / 前置定义集合（骨架扩展位）。
#[derive(Debug, Clone, Default)]
pub struct ProductionDefinitions {
    /// 条目数占位（细表后续接入）。
    pub count: u32,
}
