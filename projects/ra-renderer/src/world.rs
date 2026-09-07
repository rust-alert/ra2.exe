//! 可跨帧复用的渲染世界（R1）。
//!
//! 由 [`crate::frame::FrameBuilder`] 根据引擎只读投影 / 脏集增量更新。
//! 不持有权威 `MatchState`，不在此解码 MIX/INI。

use std::collections::HashMap;

use ra_types::EntityId;

/// 渲染侧单个可视实体（跨帧槽位，避免每帧新建整表）。
#[derive(Debug, Clone)]
pub struct RenderUnit {
    /// 稳定实体 ID。
    pub id: EntityId,
    /// 相对预览原点的屏幕像素 X。
    pub screen_x: i32,
    /// 相对预览原点的屏幕像素 Y。
    pub screen_y: i32,
    /// 是否建筑（标记形状）。
    pub is_structure: bool,
    /// 是否死亡。
    pub dead: bool,
    /// 是否本地选中。
    pub selected: bool,
}

/// 当前可视对象的渲染侧状态（跨帧复用）。
#[derive(Debug, Default)]
pub struct RenderWorld {
    /// 逻辑仿真 tick（与引擎投影对齐；插值用）。
    pub source_tick: u64,
    /// 本帧写入/更新的实体数（诊断）。
    pub dirty_count: u32,
    /// 以 `EntityId.0` 为键的可视实体槽。
    pub units: HashMap<u64, RenderUnit>,
}

impl RenderWorld {
    /// 当前槽位数。
    pub fn unit_count(&self) -> usize {
        self.units.len()
    }

    /// 清空全部槽位（重开对局时）。
    pub fn clear_units(&mut self) {
        self.units.clear();
        self.dirty_count = 0;
    }
}
