//! 间谍渗透冻结规则（效果数值来自规则表，不由 engine 常量冒充权威）。

/// 对局级渗透数值与默认效果策略。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InfiltrationRules {
    /// 渗透电厂后的断电时长（逻辑 tick）。
    pub power_blackout_ticks: u32,
    /// 渗透精炼厂时最多转走的资金。
    pub refinery_steal_funds: i32,
}

impl Default for InfiltrationRules {
    fn default() -> Self {
        // 零售 RA2 竖切缺省（adaptor 可覆盖；engine 不得再写死同名常量当权威）。
        Self { power_blackout_ticks: 300, refinery_steal_funds: 5_000 }
    }
}
