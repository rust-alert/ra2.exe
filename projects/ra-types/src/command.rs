//! 命令与事件基础载荷（跨层共享的形状，不含引擎调度细节）。

use crate::{
    id::{EntityId, PlayerId, TypeId},
    math::Cell,
    time::Tick,
};

/// 命令实例编号（日志 / 拒绝回执 / 联机去重）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
pub struct CommandId(pub u64);

/// 命令种类（扩展时保持判别式稳定，供编解码与 UI）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum CommandKind {
    /// 移动。
    MoveTo,
    /// 攻击。
    Attack,
    /// 部署。
    Deploy,
    /// 放置建筑。
    PlaceBuilding,
    /// 生产。
    Produce,
    /// 设置集结点。
    SetRally,
    /// 其它 / 扩展。
    Other,
}

/// 命令目标。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CommandTarget {
    /// 无目标。
    None,
    /// 地图格。
    Cell(Cell),
    /// 实体。
    Entity(EntityId),
    /// 类型（生产 / 放置）。
    Type(TypeId),
}

/// 已调度、带发出者与逻辑时间的命令信封（载荷形状预留）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ScheduledCommand {
    /// 命令 ID。
    pub id: CommandId,
    /// 发出玩家。
    pub player: PlayerId,
    /// 计划执行的 tick（或入队 tick）。
    pub tick: Tick,
    /// 种类。
    pub kind: CommandKind,
    /// 目标。
    pub target: CommandTarget,
}
