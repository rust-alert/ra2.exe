//! 系统调度计划。

/// 一个逻辑 tick 内的固定阶段（顺序由 [`SystemSchedule`] 决定）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SystemPhase {
    /// 消费本 tick 输入帧中的命令。
    ApplyCommands,
    /// 移动与寻路步进。
    Movement,
    /// 受击闪白倒计时。
    HitFlash,
    /// 战斗结算。
    Combat,
    /// 炮塔转向。
    Turrets,
    /// 矿场收入。
    RefineryIncome,
    /// 工厂生产推进。
    Production,
    /// 超武效果（闪电风暴倒计时与光照档切换）。
    Powers,
    /// 地图触发器与脚本动作。
    Triggers,
    /// 重算状态摘要。
    Rehash,
}

impl SystemPhase {
    /// Alpha 默认阶段顺序（确定性唯一来源）。
    pub fn default_order() -> Vec<Self> {
        vec![
            Self::ApplyCommands,
            Self::Movement,
            Self::HitFlash,
            Self::Combat,
            Self::Turrets,
            Self::RefineryIncome,
            Self::Production,
            Self::Powers,
            Self::Triggers,
            Self::Rehash,
        ]
    }
}

/// 固定系统阶段计划。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SystemSchedule {
    /// 本 tick 按序执行的阶段列表。
    pub phases: Vec<SystemPhase>,
}

impl Default for SystemSchedule {
    fn default() -> Self {
        Self { phases: SystemPhase::default_order() }
    }
}

impl SystemSchedule {
    /// 使用默认阶段顺序。
    pub fn standard() -> Self {
        Self::default()
    }

    /// 自定义阶段顺序（可省略阶段以关闭对应系统）。
    pub fn from_phases(phases: impl Into<Vec<SystemPhase>>) -> Self {
        Self { phases: phases.into() }
    }

    /// 阶段是否会出现在本计划中。
    pub fn contains(&self, phase: SystemPhase) -> bool {
        self.phases.contains(&phase)
    }

    /// 去掉指定阶段（用于启停系统；其余顺序保持不变）。
    pub fn without(mut self, phase: SystemPhase) -> Self {
        self.phases.retain(|p| *p != phase);
        self
    }
}
