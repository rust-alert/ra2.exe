//! 游戏会话公共模型类型。

/// 默认仿真频率（与渲染帧率无关）。
pub const DEFAULT_TICK_HZ: u32 = 15;

/// 单次 `pump` 最多追赶的 tick 数，防止卡顿后螺旋追帧。
pub const MAX_TICKS_PER_PUMP: u32 = 8;

/// 会话画面（供桌面流程切换，不进入 BattleState tick）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SessionScreen {
    /// 对局进行中（含暂停）。
    InBattle,
    /// 结算画面。
    Results,
}

/// 单位/建筑呈现动画状态（A0 契约首批子集）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AnimState {
    /// 待机。
    Idle,
    /// 移动。
    Move,
    /// 攻击。
    Attack,
    /// 受击闪白。
    TakeDamage,
    /// 死亡。
    Die,
    /// 工厂生产中。
    Produce,
}

/// 装载契约种类：遭遇战与战役胜负 / 开局规则不同。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum SessionBootKind {
    /// 遭遇战：剥机动、席位种 MCV、[`crate::game::BattleSession::sole_victor`] 结算。
    #[default]
    Skirmish,
    /// 战役：保留预放部队、触发器 Win/Lose 结算。
    Campaign,
}
