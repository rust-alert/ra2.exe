//! 命令与事件基础载荷（跨层共享的形状，不含引擎调度细节）。

use crate::{
    id::{EntityId, PlayerId},
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

/// 命令目标（粗粒度，供 UI / 日志；精确载荷见 [`CommandBody`]）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CommandTarget {
    /// 无目标。
    None,
    /// 地图格。
    Cell {
        /// 格 X。
        x: u16,
        /// 格 Y。
        y: u16,
    },
    /// 实体。
    Entity(EntityId),
    /// 外部类型键（生产 / 放置）。
    TypeKey(String),
}

/// 可执行命令体（跨桌面 / 测试 / 网络的共同载荷，不含调度信封）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CommandBody {
    /// 将实体移动到目标格。
    MoveTo {
        /// 实体稳定 ID。
        entity: EntityId,
        /// 目标格 X。
        x: u16,
        /// 目标格 Y。
        y: u16,
    },
    /// 指定攻击目标。
    Attack {
        /// 攻击方。
        attacker: EntityId,
        /// 被攻击方。
        target: EntityId,
    },
    /// 部署可展开实体。
    Deploy {
        /// 实体稳定 ID。
        entity: EntityId,
    },
    /// 在目标格放置建筑。
    PlaceBuilding {
        /// 出资并拥有该建筑的玩家。
        player: PlayerId,
        /// 外部类型键。
        type_id: String,
        /// 目标格 X。
        x: u16,
        /// 目标格 Y。
        y: u16,
    },
    /// 在空闲工厂排队生产单位。
    Produce {
        /// 出资玩家。
        player: PlayerId,
        /// 外部类型键。
        type_id: String,
    },
    /// 为工厂设置生产集结点。
    SetRallyPoint {
        /// 工厂实体稳定 ID。
        factory: EntityId,
        /// 集结格 X。
        x: u16,
        /// 集结格 Y。
        y: u16,
    },
}

impl CommandBody {
    /// 对应的粗粒度种类。
    pub fn kind(&self) -> CommandKind {
        match self {
            Self::MoveTo { .. } => CommandKind::MoveTo,
            Self::Attack { .. } => CommandKind::Attack,
            Self::Deploy { .. } => CommandKind::Deploy,
            Self::PlaceBuilding { .. } => CommandKind::PlaceBuilding,
            Self::Produce { .. } => CommandKind::Produce,
            Self::SetRallyPoint { .. } => CommandKind::SetRally,
        }
    }

    /// 对应的粗粒度主目标（攻击取目标方，移动取格子）。
    pub fn primary_target(&self) -> CommandTarget {
        match self {
            Self::MoveTo { x, y, .. } | Self::SetRallyPoint { x, y, .. } | Self::PlaceBuilding { x, y, .. } => {
                CommandTarget::Cell { x: *x, y: *y }
            }
            Self::Attack { target, .. } => CommandTarget::Entity(*target),
            Self::Deploy { entity } => CommandTarget::Entity(*entity),
            Self::Produce { type_id, .. } => CommandTarget::TypeKey(type_id.clone()),
        }
    }
}

/// 已调度、带发出者与逻辑时间的命令信封。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ScheduledCommand {
    /// 命令 ID。
    pub id: CommandId,
    /// 发出玩家。
    pub player: PlayerId,
    /// 计划执行的 tick（或入队时的下一消费 tick）。
    pub tick: Tick,
    /// 可执行载荷。
    pub body: CommandBody,
}

impl ScheduledCommand {
    /// 构造信封。
    pub fn new(id: CommandId, player: PlayerId, tick: Tick, body: CommandBody) -> Self {
        Self { id, player, tick, body }
    }

    /// 粗粒度种类。
    pub fn kind(&self) -> CommandKind {
        self.body.kind()
    }

    /// 粗粒度主目标。
    pub fn target(&self) -> CommandTarget {
        self.body.primary_target()
    }
}
