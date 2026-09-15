//! 玩法侧 ECS 组件（无 INI / MIX 依赖）。
//!
//! 运行时玩法字段以 ECS 为权威。`WorldEntity` 仅为投影槽，经 `project_entity_from_ecs` 覆盖。

use ra_map::MapEntityKind;
use ra_types::{ArmorKind, EntityId, HouseId, MissionKind, TagId, TechnoClass, TypeId};

/// 稳定身份与内容类型引用。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Identity {
    /// 对外稳定实体 ID。
    pub entity_id: EntityId,
    /// 内容类型稳定 id（与定义集对应）。
    pub type_id: TypeId,
    /// 地图实体种类。
    pub kind: MapEntityKind,
    /// 地图放置 / 运行时任务态；`None` 表示未指定。
    pub mission: Option<MissionKind>,
    /// 绑定的地图 Tag 稳定 id；`None` 表示无 Tag。
    pub tag: Option<TagId>,
}

/// 所属房主。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Owner {
    /// 房主稳定 id（与定义集对应）。
    pub house: HouseId,
}

/// 格坐标、朝向与子格。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Transform {
    /// 格坐标 X。
    pub x: u16,
    /// 格坐标 Y。
    pub y: u16,
    /// 车身朝向（0..=255）。
    pub facing: u8,
    /// 炮塔朝向。
    pub turret_facing: u8,
    /// 子格偏移。
    pub sub_cell: u8,
}

/// 生命与死亡标记。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Health {
    /// 当前生命。
    pub current: u32,
    /// 最大生命。
    pub maximum: u32,
    /// 是否已逻辑死亡。
    pub dead: bool,
}

/// 侧栏扳手修理中：按 `[General]` 的 `RepairRate` / `RepairStep` / `RepairPercent` 持续回血扣费。
///
/// 由 `RepairBuilding` 命令挂上或摘下；回满或资金不足时由修理步进摘除。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Repairing;

/// 定时无敌（铁幕／力场护盾等）；剩余 tick 归零后摘除。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TimedInvulnerability {
    /// 剩余无敌逻辑 tick。
    pub remaining_ticks: u32,
}

/// 移动能力（速度等，来自冻结定义播种）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Locomotor {
    /// 每 tick 累加的移动点。
    pub speed: u32,
}

/// 当前移动路径与目的地。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MovementState {
    /// 移动目标格 X。
    pub destination_x: Option<u16>,
    /// 移动目标格 Y。
    pub destination_y: Option<u16>,
    /// 后续航点（抵达当前目的地后依次弹出为新目的地；路径点规划用）。
    pub waypoints: Vec<(u16, u16)>,
    /// 剩余路径。
    pub path: Vec<(u16, u16)>,
    /// 累积移动点。
    pub move_accum: u32,
}

/// 武器与护甲的静态战斗参数（定义播种，对局中很少改）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CombatStats {
    /// 护甲种类。
    pub armor: ArmorKind,
    /// 攻击射程。
    pub attack_range: u32,
    /// 单次基础伤害。
    pub attack_damage: u32,
    /// 开火冷却上限。
    pub attack_cooldown_max: u32,
    /// 弹头对各护甲的伤害百分比。
    pub attack_verses: [u32; 11],
    /// 对应 techno 种类。
    pub techno_class: Option<TechnoClass>,
}

/// 攻击目标与开火冷却。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AttackState {
    /// 攻击目标实体。
    pub target: Option<EntityId>,
    /// 开火冷却剩余 tick。
    pub cooldown: u32,
    /// 渗透目标建筑（间谍专用；有值时不走普通开火）。
    pub infiltrate_target: Option<EntityId>,
    /// 占领目标建筑（工程师专用；有值时不走普通开火）。
    pub capture_target: Option<EntityId>,
    /// 跟随目标实体（`mission=Follow` 时持续追格）。
    pub follow_target: Option<EntityId>,
}

/// 单位厂 FIFO 总长上限（队首 `item` + `pending`），对齐原版侧栏可连点排队的量级。
pub const MAX_UNIT_QUEUE_LEN: usize = 30;

/// 单槽在产项：边造边扣，取消退已扣部分。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProductionSlot {
    /// 稳定类型 id。
    pub type_id: TypeId,
    /// 剩余推进 tick（没钱时不递减）。
    pub remaining_ticks: u32,
    /// 开工时总 tick（按比例扣款，末步补齐 `Cost`）。
    pub total_ticks: u32,
    /// 本项已从资金扣除的数额。
    pub paid: i32,
}

impl ProductionSlot {
    /// 新开单项：尚未扣款，剩余 = 总 tick。
    pub fn new(type_id: TypeId, total_ticks: u32) -> Self {
        let total_ticks = total_ticks.max(1);
        Self { type_id, remaining_ticks: total_ticks, total_ticks, paid: 0 }
    }
}

/// 工厂生产队列与集结格。
///
/// - **单位厂**：`item` 为队首推进项，`pending` 为 FIFO 候补；无 `ready`。
/// - **建造场**：建筑栏用 `item`/`ready`，防御栏（`BuildCat=Combat`）用 `defense_item`/`defense_ready`，
///   两轨并发；结构队列无 backlog（完工待落位占槽，落位或取消后才可再开单）。
/// - **扣款**：队首按 tick 边造边扣；候补未开工不扣；没钱则暂停推进。
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct ProductionQueue {
    /// 建筑栏 / 单位厂队首。
    pub item: Option<ProductionSlot>,
    /// 建造场防御栏队首（与 `item` 并发；单位厂不用）。
    pub defense_item: Option<ProductionSlot>,
    /// 建造场建筑栏已完工、待点选落位的稳定类型 id。
    pub ready: Option<TypeId>,
    /// 建造场防御栏已完工、待点选落位的稳定类型 id。
    pub defense_ready: Option<TypeId>,
    /// 单位厂 FIFO 候补（不含队首）；建造场两侧栏不用。
    pub pending: Vec<TypeId>,
    /// 集结格 X。
    pub rally_x: Option<u16>,
    /// 集结格 Y。
    pub rally_y: Option<u16>,
    /// 是否为本房主、本生产类别的主厂（PRI）。
    pub is_primary: bool,
}

impl ProductionQueue {
    /// 空队列（无在产、无完工件、无候补、无集结）。
    pub fn empty() -> Self {
        Self::default()
    }

    /// 清空全部在产 / 完工件 / 候补（保留集结格）。
    pub fn clear_production(&mut self) {
        self.item = None;
        self.defense_item = None;
        self.ready = None;
        self.defense_ready = None;
        self.pending.clear();
    }

    /// 单位厂当前排队长度（队首 + 候补）。
    pub fn unit_len(&self) -> usize {
        usize::from(self.item.is_some()) + self.pending.len()
    }

    /// 单位厂是否还能再入队一件。
    pub fn can_enqueue_unit(&self) -> bool {
        self.unit_len() < MAX_UNIT_QUEUE_LEN
    }

    /// 建造场对应轨是否占用中（在产或待落位）。
    pub fn structure_track_busy(&self, defense: bool) -> bool {
        if defense { self.defense_item.is_some() || self.defense_ready.is_some() } else { self.item.is_some() || self.ready.is_some() }
    }

    /// 任一槽位是否持有该类型（在产 / 待落位 / 候补）。
    pub fn holds_type(&self, type_id: TypeId) -> bool {
        self.item.as_ref().is_some_and(|s| s.type_id == type_id)
            || self.defense_item.as_ref().is_some_and(|s| s.type_id == type_id)
            || self.ready == Some(type_id)
            || self.defense_ready == Some(type_id)
            || self.pending.iter().any(|&t| t == type_id)
    }

    /// 是否有任意在产项（含防御轨）。
    #[allow(dead_code)]
    pub fn is_producing(&self) -> bool {
        self.item.is_some() || self.defense_item.is_some()
    }
}

/// 采矿车采集 / 运载状态。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct HarvesterState {
    /// 站在矿格上采集累计 tick。
    pub ore_trip_accum: u32,
    /// 已装载趟数（竖切：`0` 空载，`1` 满载可卸）。
    pub cargo: u8,
}

/// 建筑周期产钱累计（`ProduceCashDelay`）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct CashProducerState {
    /// 距下一次 `ProduceCashAmount` 发放已累计的 tick。
    pub accum: u32,
}

/// 呈现相关动画桥接状态。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AnimationState {
    /// HVA 动画帧。
    pub hva_frame: u16,
    /// 受击闪白剩余 tick。
    pub hit_flash: u32,
    /// 开火呈现剩余 tick（步兵 `Fire` 序列等）。
    pub fire_flash: u32,
}

/// `Deployer` 就地姿态：美国大兵等蹲姿 / 展开；与 `DeploysInto` 建筑展开无关。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct DeployStance {
    /// 是否处于已部署姿态。
    pub deployed: bool,
}

/// 生成时一次性写入的 ECS 组件包（权威），随后投影到 `WorldEntity` 槽位。
#[derive(Debug, Clone)]
pub struct EntitySpawnBundle {
    /// 稳定身份。
    pub identity: Identity,
    /// 所属房主。
    pub owner: Owner,
    /// 空间变换。
    pub transform: Transform,
    /// 生命。
    pub health: Health,
    /// 移动能力。
    pub locomotor: Locomotor,
    /// 移动状态。
    pub movement: MovementState,
    /// 战斗静态参数。
    pub combat: CombatStats,
    /// 攻击状态。
    pub attack: AttackState,
    /// 生产队列。
    pub production: ProductionQueue,
    /// 采矿状态。
    pub harvester: HarvesterState,
    /// 周期产钱累计。
    pub cash_producer: CashProducerState,
    /// 动画桥接。
    pub animation: AnimationState,
    /// 就地部署姿态。
    pub deploy_stance: DeployStance,
}
