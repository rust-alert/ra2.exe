//! 命令拒绝原因。供会话与 HUD 展示确定反馈。

/// 世界拒绝执行某条命令时的原因。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CommandRejectReason {
    /// 实体下标或 ID 无效。
    EntityNotFound,
    /// 实体已死亡。
    EntityDead,
    /// 该实体不能移动或执行该移动类命令。
    NotMobile,
    /// 攻击目标非法。
    InvalidTarget,
    /// 该实体不能部署。
    CannotDeploy,
    /// 资金不足。
    InsufficientFunds,
    /// 供电不足。
    InsufficientPower,
    /// 缺少前置建筑或科技。
    MissingPrerequisite,
    /// 无法在目标格放置。
    InvalidPlacement,
    /// 操作了非己方实体。
    WrongOwner,
    /// 生产或建造队列已满。
    QueueFull,
    /// 对局已结束。
    MatchEnded,
}

impl CommandRejectReason {
    /// 给 HUD / 标题栏用的简短中文文案。
    pub fn as_hud_label(self) -> &'static str {
        match self {
            Self::EntityNotFound => "目标不存在",
            Self::EntityDead => "目标已阵亡",
            Self::NotMobile => "无法移动",
            Self::InvalidTarget => "无效目标",
            Self::CannotDeploy => "无法部署",
            Self::InsufficientFunds => "资金不足",
            Self::InsufficientPower => "电力不足",
            Self::MissingPrerequisite => "前置不足",
            Self::InvalidPlacement => "无法放置",
            Self::WrongOwner => "非己方单位",
            Self::QueueFull => "队列已满",
            Self::MatchEnded => "对局已结束",
        }
    }
}

/// 一条被拒绝的命令记录（按 tick 内命令序）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CommandReject {
    /// 本 tick 输入帧中的命令下标。
    pub command_index: usize,
    /// 拒绝原因。
    pub reason: CommandRejectReason,
}
