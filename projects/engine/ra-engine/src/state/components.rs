//! 玩法侧 ECS 组件（无 INI / MIX 依赖）。
//!
//! 迁移约定：多数字段仍由 `WorldEntity` 权威推进，并在 tick 末同步到组件。
//! `Health` 已改为 ECS 权威，经投影写回 `WorldEntity` 供未迁移读路径使用。

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
}

/// 工厂生产队列与集结格。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProductionQueue {
    /// 队列中的类型 ID 与剩余 tick。
    pub item: Option<(Arc<str>, u32)>,
    /// 集结格 X。
    pub rally_x: Option<u16>,
    /// 集结格 Y。
    pub rally_y: Option<u16>,
}

/// 矿场采矿行程进度。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct HarvesterState {
    /// 本趟采矿累计 tick。
    pub ore_trip_accum: u32,
}

/// 呈现相关动画桥接状态。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AnimationState {
    /// HVA 动画帧。
    pub hva_frame: u16,
    /// 受击闪白剩余 tick。
    pub hit_flash: u32,
}
