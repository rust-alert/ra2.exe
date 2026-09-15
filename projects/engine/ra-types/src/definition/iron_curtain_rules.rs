//! 铁幕 / 力场护盾全局规则。

/// 铁幕类超武执行参数（装载期冻结）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IronCurtainRules {
    /// 无敌持续 tick。
    pub duration_ticks: u32,
    /// 作用半径（切比雪夫格数，含中心）。
    pub radius_cells: u32,
}

impl Default for IronCurtainRules {
    fn default() -> Self {
        // 竖切缺省：约数秒级无敌、小半径。
        Self { duration_ticks: 450, radius_cells: 3 }
    }
}
