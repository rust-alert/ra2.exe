//! 玩法侧 ECS 组件（无 INI / MIX 依赖）。
//!
//! 当前仍由 `WorldEntity` 权威推进，tick 末同步到这些组件，供迁移期查询与后续翻转权威。

use std::sync::Arc;

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
