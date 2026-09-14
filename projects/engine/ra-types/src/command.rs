//! 命令与事件基础载荷（跨层共享的形状，不含引擎调度细节）。

use crate::{
    id::{EntityId, PlayerId, TypeId},
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
    /// 沿多航点移动（路径点规划）。
    MovePath,
    /// 攻击。
    Attack,
    /// 部署。
    Deploy,
    /// 放置建筑。
    PlaceBuilding,
    /// 生产。
    Produce,
    /// 取消生产。
    CancelProduce,
    /// 设置集结点。
    SetRally,
    /// 将生产厂设为本房主该生产类别的主厂（PRI）。
    SetPrimary,
    /// 间谍渗透敌方建筑。
    Infiltrate,
    /// 工程师占领敌方可俘建筑。
    CaptureBuilding,
    /// 就地警戒（清移动/攻击目标，写入 `mission=Guard`）。
    Guard,
    /// 停止：清移动与攻击目标，并清空 `mission`。
    Stop,
    /// 攻击移动：向目标格移动，途中自动接敌。
    AttackMove,
    /// 跟随：持续追某一实体当前格（`mission=Follow`）。
    Follow,
    /// 散开：向邻近空闲格短距移动以解除叠压。
    Scatter,
    /// 删除：摧毁己方选中单位或建筑（无退款）。
    Delete,
    /// 出售己方建筑（侧栏出售工具）。
    SellBuilding,
    /// 切换己方建筑的持续修理（侧栏修理工具；对应原版扳手挂/摘修理）。
    RepairBuilding,
    /// 释放超级武器（选点）。
    FireSuperWeapon,
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
    /// 冻结规则中的类型 ID（生产 / 放置）。
    TypeId(TypeId),
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
    /// 沿航点序列移动：首点为当前目的地，其余入 `MovementState.waypoints`。
    MovePath {
        /// 实体稳定 ID。
        entity: EntityId,
        /// 航点格序列（至少一点）。
        points: Vec<(u16, u16)>,
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
        /// 冻结规则中的类型 ID。
        type_id: TypeId,
        /// 目标格 X。
        x: u16,
        /// 目标格 Y。
        y: u16,
    },
    /// 在空闲工厂排队生产单位。
    Produce {
        /// 出资玩家。
        player: PlayerId,
        /// 冻结规则中的类型 ID。
        type_id: TypeId,
    },
    /// 取消本方工厂中指定类型的在产项并退款。
    CancelProduce {
        /// 出资玩家。
        player: PlayerId,
        /// 冻结规则中的类型 ID。
        type_id: TypeId,
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
    /// 将工厂设为本房主、本生产类别唯一主厂（PRI）。
    SetPrimaryFactory {
        /// 工厂实体稳定 ID。
        factory: EntityId,
    },
    /// 间谍渗透敌方建筑。
    Infiltrate {
        /// 间谍实体。
        agent: EntityId,
        /// 目标建筑实体。
        building: EntityId,
    },
    /// 工程师占领敌方可俘建筑。
    CaptureBuilding {
        /// 工程师实体。
        engineer: EntityId,
        /// 目标建筑实体。
        building: EntityId,
    },
    /// 就地警戒：清空移动与攻击目标，并设置 `Identity.mission` 为 `Guard`。
    Guard {
        /// 实体稳定 ID。
        entity: EntityId,
    },
    /// 停止：清空移动与攻击目标，并清空 `Identity.mission`。
    Stop {
        /// 实体稳定 ID。
        entity: EntityId,
    },
    /// 攻击移动：向目标格移动，途中自动接敌（`mission=AttackMove`）。
    AttackMove {
        /// 实体稳定 ID。
        entity: EntityId,
        /// 目标格 X。
        x: u16,
        /// 目标格 Y。
        y: u16,
    },
    /// 跟随：指定跟随目标（`mission=Follow`，`AttackState.follow_target`）。
    Follow {
        /// 跟随方实体。
        entity: EntityId,
        /// 被跟随实体。
        target: EntityId,
    },
    /// 散开：清攻击目标并向邻近空闲格短距移动。
    Scatter {
        /// 实体稳定 ID。
        entity: EntityId,
    },
    /// 删除：立即摧毁己方实体（无退款；对应 `Delete` 热键）。
    Delete {
        /// 实体稳定 ID。
        entity: EntityId,
    },
    /// 出售己方建筑：按残血比例退半价，写入 `Selling` 并排队拆除呈现。
    SellBuilding {
        /// 出资并拥有该建筑的玩家。
        player: PlayerId,
        /// 目标建筑实体。
        building: EntityId,
    },
    /// 切换己方建筑持续修理：挂上或摘下修理标记，由仿真步进按规则回血扣费。
    RepairBuilding {
        /// 出资并拥有该建筑的玩家。
        player: PlayerId,
        /// 目标建筑实体。
        building: EntityId,
    },
    /// 释放超级武器到目标格（须本方充能就绪且仍有挂接该超武的存活建筑）。
    FireSuperWeapon {
        /// 释放玩家。
        player: PlayerId,
        /// 超级武器稳定类型编号（`[SuperWeaponTypes]` 冻结 id）。
        type_id: TypeId,
        /// 目标格 X。
        x: u16,
        /// 目标格 Y。
        y: u16,
    },
}

impl CommandBody {
    /// 对应的粗粒度种类。
    pub fn kind(&self) -> CommandKind {
        match self {
            Self::MoveTo { .. } => CommandKind::MoveTo,
            Self::MovePath { .. } => CommandKind::MovePath,
            Self::Attack { .. } => CommandKind::Attack,
            Self::Deploy { .. } => CommandKind::Deploy,
            Self::PlaceBuilding { .. } => CommandKind::PlaceBuilding,
            Self::Produce { .. } => CommandKind::Produce,
            Self::CancelProduce { .. } => CommandKind::CancelProduce,
            Self::SetRallyPoint { .. } => CommandKind::SetRally,
            Self::SetPrimaryFactory { .. } => CommandKind::SetPrimary,
            Self::Infiltrate { .. } => CommandKind::Infiltrate,
            Self::CaptureBuilding { .. } => CommandKind::CaptureBuilding,
            Self::Guard { .. } => CommandKind::Guard,
            Self::Stop { .. } => CommandKind::Stop,
            Self::AttackMove { .. } => CommandKind::AttackMove,
            Self::Follow { .. } => CommandKind::Follow,
            Self::Scatter { .. } => CommandKind::Scatter,
            Self::Delete { .. } => CommandKind::Delete,
            Self::SellBuilding { .. } => CommandKind::SellBuilding,
            Self::RepairBuilding { .. } => CommandKind::RepairBuilding,
            Self::FireSuperWeapon { .. } => CommandKind::FireSuperWeapon,
        }
    }

    /// 对应的粗粒度主目标（攻击取目标方，移动取格子）。
    pub fn primary_target(&self) -> CommandTarget {
        match self {
            Self::MoveTo { x, y, .. }
            | Self::AttackMove { x, y, .. }
            | Self::SetRallyPoint { x, y, .. }
            | Self::PlaceBuilding { x, y, .. }
            | Self::FireSuperWeapon { x, y, .. } => CommandTarget::Cell { x: *x, y: *y },
            Self::MovePath { points, .. } => match points.first() {
                Some(&(x, y)) => CommandTarget::Cell { x, y },
                None => CommandTarget::None,
            },
            Self::Attack { target, .. } | Self::Follow { target, .. } => CommandTarget::Entity(*target),
            Self::Infiltrate { building, .. }
            | Self::CaptureBuilding { building, .. }
            | Self::SellBuilding { building, .. }
            | Self::RepairBuilding { building, .. } => CommandTarget::Entity(*building),
            Self::Deploy { entity }
            | Self::Guard { entity }
            | Self::Stop { entity }
            | Self::Scatter { entity }
            | Self::Delete { entity }
            | Self::SetPrimaryFactory { factory: entity } => CommandTarget::Entity(*entity),
            Self::Produce { type_id, .. } | Self::CancelProduce { type_id, .. } => CommandTarget::TypeId(*type_id),
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
