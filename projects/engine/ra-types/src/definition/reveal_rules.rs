//! 地图揭示类超武 / 触发全局规则。

/// 揭示圆盘执行参数（装载期冻结）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RevealRules {
    /// 揭示半径（切比雪夫格数，含中心；对应 `RevealTriggerRadius`）。
    pub radius_cells: u32,
}

impl Default for RevealRules {
    fn default() -> Self {
        // 零售 `[General] RevealTriggerRadius` 缺省为 9。
        Self { radius_cells: 9 }
    }
}
