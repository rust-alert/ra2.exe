//! 生产队列与前置定义表。

/// 生产 / 前置定义集合（骨架）。
#[derive(Debug, Clone, Default)]
pub struct ProductionDefinitions {
    /// 条目数占位。
    pub count: u32,
}
