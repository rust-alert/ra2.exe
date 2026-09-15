//! 闪电风暴全局规则（`[General]` 相关键），供超武执行器读取。

/// 闪电风暴执行参数（装载期冻结；缺省对齐当前竖切）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LightningStormRules {
    /// 激活后持续 tick（原版 `LightningStormDuration` 语义的冻结值）。
    pub duration_ticks: u32,
    /// 释放后延迟激活 tick（原版 `LightningDeferment`）。
    pub deferment_ticks: u32,
}

impl Default for LightningStormRules {
    fn default() -> Self {
        Self { duration_ticks: 90, deferment_ticks: 0 }
    }
}
