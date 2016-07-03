//! 玩法侧 ECS 组件（无 INI / MIX 依赖）。
//!
//! 运行时玩法字段以 ECS 为权威。`WorldEntity` 仅为投影槽，经 `project_entity_from_ecs` 覆盖。

use std::sync::Arc;

use ra_assets::TechnoKind;
use ra_map::MapEntityKind;
use ra_types::EntityId;

/// 稳定身份与内容类型引用。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Identity {
    /// 对外稳定实体 ID。
    pub entity_id: EntityId,
    /// 内容类型键（与定义集对应）。
    pub type_id: Arc<str>,
    /// 地图实体种类。
    pub kind: MapEntityKind,
    /// 地图放置任务态（如 `Guard` / `Attack`；空表示未指定）。
    pub mission: String,
    /// 地图 Tag id（空表示无绑定）。
    pub tag: String,
}

/// 所属房主。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Owner {
    /// 房主名称（跨帧共享）。
    pub house: Arc<str>,
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
    /// 护甲名。
    pub armor: String,
    /// 攻击射程。
    pub attack_range: u32,
    /// 单次基础伤害。
    pub attack_damage: u32,
    /// 开火冷却上限。
    pub attack_cooldown_max: u32,
    /// 弹头对各护甲的伤害百分比。
    pub attack_verses: [u32; 11],
    /// 对应 techno 种类。
    pub techno_kind: Option<TechnoKind>,
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
}

/// 工厂生产队列与集结格。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProductionQueue {
    /// 队列中的类型 ID 与剩余 tick（建造中）。
    pub item: Option<(Arc<str>, u32)>,
    /// 建造场已完工、待点选落位的建筑类型（单位厂不用）。
    pub ready: Option<Arc<str>>,
    /// 集结格 X。
    pub rally_x: Option<u16>,
    /// 集结格 Y。
    pub rally_y: Option<u16>,
}

/// 采矿车采集 / 运载状态。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct HarvesterState {
    /// 站在矿格上采集累计 tick。
    pub ore_trip_accum: u32,
    /// 已装载趟数（竖切：`0` 空载，`1` 满载可卸）。
    pub cargo: u8,
}

/// 呈现相关动画桥接状态。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AnimationState {
    /// HVA 动画帧。
    pub hva_frame: u16,
    /// 受击闪白剩余 tick。
    pub hit_flash: u32,
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
    /// 动画桥接。
    pub animation: AnimationState,
}
