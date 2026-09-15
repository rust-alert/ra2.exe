//! 箱子奖励冻结规则。

/// 对局级箱子奖励表（装载期冻结）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CrateRules {
    /// 脚本 / 触发器生成箱子的默认金钱奖励（`[CrateRules] CrateMoney` 或缺省）。
    pub default_credits: i32,
    /// 金钱箱下限（若规则提供）。
    pub money_minimum: Option<i32>,
    /// 金钱箱上限（若规则提供）。
    pub money_maximum: Option<i32>,
}

impl Default for CrateRules {
    fn default() -> Self {
        Self { default_credits: Self::DEFAULT_CREDITS, money_minimum: None, money_maximum: None }
    }
}

impl CrateRules {
    /// 零售竖切缺省金钱（与历史 `SCRIPT_CRATE_CREDITS` 对齐）。
    pub const DEFAULT_CREDITS: i32 = 2_000;

    /// 结算一次金钱奖励（夹在可选上下限内）。
    pub fn resolve_credits(&self, preferred: Option<i32>) -> i32 {
        let mut amount = preferred.unwrap_or(self.default_credits).max(0);
        if let Some(min) = self.money_minimum {
            amount = amount.max(min.max(0));
        }
        if let Some(max) = self.money_maximum {
            amount = amount.min(max.max(0));
        }
        amount
    }
}
