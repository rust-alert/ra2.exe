//! 间谍渗透冻结规则（效果数值与建筑效果档案来自规则投影，不由 engine 常量冒充权威）。

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

/// 单栋建筑被间谍渗透后的效果类别（装载期冻结；engine 只按效果执行）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum InfiltrationEffect {
    /// 无特殊效果（仍播通用 EVA）。
    #[default]
    Generic,
    /// 断电（时长取自 [`InfiltrationRules::power_blackout_ticks`]）。
    PowerBlackout,
    /// 偷取资金（上限取自 [`InfiltrationRules::refinery_steal_funds`]）。
    StealFunds,
    /// 解锁步兵老兵生产。
    PromoteInfantry,
    /// 解锁载具老兵生产。
    PromoteVehicle,
    /// 授予受害方阵营对应的偷取科技。
    StealTech,
}

/// 建筑渗透档案（由 adaptor 从能力／生产类别／科技建筑组投影；后续可接扩展 INI）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct InfiltrationProfile {
    /// 渗透生效类别。
    pub effect: InfiltrationEffect,
}

impl InfiltrationProfile {
    /// 构造指定效果。
    pub const fn new(effect: InfiltrationEffect) -> Self {
        Self { effect }
    }

    /// 由建筑能力／生产类别／是否科技建筑冻结档案（优先级对齐原版启发式）。
    pub fn freeze_from_stock(
        is_power_producer: bool,
        is_refinery: bool,
        production_category: Option<super::ProductionCategory>,
        is_tech_building: bool,
    ) -> Self {
        if is_power_producer {
            return Self::new(InfiltrationEffect::PowerBlackout);
        }
        if is_refinery {
            return Self::new(InfiltrationEffect::StealFunds);
        }
        if let Some(category) = production_category {
            return match category {
                super::ProductionCategory::Infantry => Self::new(InfiltrationEffect::PromoteInfantry),
                super::ProductionCategory::Vehicle => Self::new(InfiltrationEffect::PromoteVehicle),
                super::ProductionCategory::Aircraft | super::ProductionCategory::Building => Self::new(InfiltrationEffect::Generic),
            };
        }
        if is_tech_building {
            return Self::new(InfiltrationEffect::StealTech);
        }
        Self::new(InfiltrationEffect::Generic)
    }
}
