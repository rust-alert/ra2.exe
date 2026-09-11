//! 世界实体的运行时投影。
//!
//! 权威状态在 ECS 组件中。本结构是兼容槽位：生成时占位，由 `project_entity_from_ecs`
//! 与 `with_*_mut` 覆盖字段。玩法决策与锁步摘要应读 ECS，勿把本结构当可变真相。

use std::sync::Arc;

use ra_map::MapEntityKind;
use ra_types::EntityId;
use ra_types::TechnoClass;

/// 实体的 ECS → 投影缓存（字段与组件对应，非权威存储）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct WorldEntity {
    /// 稳定实体 ID（不随列表紧凑化改变）。
    pub id: EntityId,
    /// 地图实体种类（单位、建筑、步兵等）。
    pub kind: MapEntityKind,
    /// 所属方名称（跨帧以 `Arc` 共享，避免每显示帧整串拷贝）。
    pub owner: Arc<str>,
    /// 外部类型键（跨帧以 `Arc` 共享）。
    pub type_id: Arc<str>,
    /// 当前格坐标。
    pub x: u16,
    /// 当前格坐标。
    pub y: u16,
    /// 车身朝向（0..=255 环）。
    pub facing: u8,
    /// 炮塔朝向（无炮塔时与 `facing` 同步）。
    pub turret_facing: u8,
    /// 子格偏移（步兵等）。
    pub sub_cell: u8,
    /// 当前生命。
    pub health: u32,
    /// 最大生命。
    pub max_health: u32,
    /// 每 tick 累加的移动速度。
    pub speed: u32,
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
    pub techno_class: Option<TechnoClass>,
    /// 移动目标格。
    pub target_x: Option<u16>,
    /// 移动目标格。
    pub target_y: Option<u16>,
    /// 剩余路径。
    pub path: Vec<(u16, u16)>,
    /// 累积移动点。
    pub move_accum: u32,
    /// HVA 动画帧。
    pub hva_frame: u16,
    /// 攻击目标实体的稳定标识。
    pub attack_target: Option<EntityId>,
    /// 开火冷却剩余 tick。
    pub attack_cooldown: u32,
    /// 矿场采矿行程累计 tick。
    pub ore_trip_accum: u32,
    /// 生产队列：（类型 ID，剩余 tick）。
    pub produce_queue: Option<(Arc<str>, u32)>,
    /// 生产集结格。
    pub rally_x: Option<u16>,
    /// 生产集结格。
    pub rally_y: Option<u16>,
    /// 受击闪白剩余 tick。
    pub hit_flash: u32,
    /// 生命归零后为真。
    pub dead: bool,
}

impl WorldEntity {
    /// 仅占投影槽位的空实体（字段由随后的 ECS 投影覆盖）。
    pub(crate) fn projection_slot(id: EntityId) -> Self {
        Self {
            id,
            kind: MapEntityKind::Unit,
            owner: Arc::from(""),
            type_id: Arc::from(""),
            x: 0,
            y: 0,
            facing: 0,
            turret_facing: 0,
            sub_cell: 0,
            health: 0,
            max_health: 0,
            speed: 0,
            armor: String::new(),
            attack_range: 0,
            attack_damage: 0,
            attack_cooldown_max: 0,
            attack_verses: [0; 11],
            techno_class: None,
            target_x: None,
            target_y: None,
            path: Vec::new(),
            move_accum: 0,
            hva_frame: 0,
            attack_target: None,
            attack_cooldown: 0,
            ore_trip_accum: 0,
            produce_queue: None,
            rally_x: None,
            rally_y: None,
            hit_flash: 0,
            dead: false,
        }
    }
}
