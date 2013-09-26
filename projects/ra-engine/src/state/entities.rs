//! 世界实体的运行时状态。

use ra_assets::TechnoKind;
use ra_map::MapEntityKind;
use ra_types::EntityId;

/// 世界中的一个已放置或生产的实体。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorldEntity {
    /// 稳定实体 ID（不随列表紧凑化改变）。
    pub id: EntityId,
    /// 地图实体种类（单位、建筑、步兵等）。
    pub kind: MapEntityKind,
    /// 所属方名称（地图放置段字符串）。
    pub owner: String,
    /// 外部类型键（来自冻结定义）。
    pub type_id: String,
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
    pub techno_kind: Option<TechnoKind>,
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
    pub produce_queue: Option<(String, u32)>,
    /// 生产集结格。
    pub rally_x: Option<u16>,
    /// 生产集结格。
    pub rally_y: Option<u16>,
    /// 受击闪白剩余 tick。
    pub hit_flash: u32,
    /// 生命归零后为真。
    pub dead: bool,
}
