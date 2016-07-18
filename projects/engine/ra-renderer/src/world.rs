//! 可跨帧复用的渲染世界（R1）。
//!
//! 由 [`crate::frame::FrameBuilder`] 根据引擎只读投影 / 脏集增量更新。
//! 不持有权威 `BattleState`，不在此解码 MIX/INI。

use std::collections::HashMap;

use ra_types::EntityId;

/// 渲染侧单个可视实体（跨帧槽位，避免每帧新建整表）。
#[derive(Debug, Clone)]
pub struct RenderUnit {
    /// 稳定实体 ID。
    pub id: EntityId,
    /// 相对预览原点的屏幕像素 X（等距格子包围盒原点）。
    pub screen_x: i32,
    /// 相对预览原点的屏幕像素 Y。
    pub screen_y: i32,
    /// 是否建筑。
    pub is_structure: bool,
    /// 是否步兵（影响 `pipbrd` 帧与 pip 段数）。
    pub is_infantry: bool,
    /// 是否死亡。
    pub dead: bool,
    /// 是否本地选中。
    pub selected: bool,
    /// 是否本帧悬停（未选中时只画血 pip）。
    pub hovered: bool,
    /// 是否可部署（选中时画部署标记）。
    pub deployable: bool,
    /// 移动最终目标锚点（预览图坐标，已含菱形中心偏移；供目标线终点）。
    pub move_goal_screen: Option<(i32, i32)>,
    /// 攻击目标锚点（预览图坐标，已含菱形中心偏移；供目标线终点）。
    pub attack_target_screen: Option<(i32, i32)>,
    /// 已烘焙的标记颜色（含阵营哈希与动画着色），避免绘制时再读字符串。
    pub color: [f32; 4],
    /// 当前生命。
    pub health: u32,
    /// 最大生命。
    pub max_health: u32,
    /// 建筑占地宽（格）。
    pub foundation_w: u16,
    /// 建筑占地高（格）。
    pub foundation_h: u16,
    /// 建筑 Height。
    pub art_height: u16,
    /// 选中血条竖直偏移。
    pub bracket_delta: i32,
}

/// 当前可视对象的渲染侧状态（跨帧复用）。
#[derive(Debug, Default)]
pub struct RenderWorld {
    /// 逻辑仿真 tick（与引擎投影对齐；插值用）。
    pub source_tick: u64,
    /// 本帧写入/更新的实体数（诊断）。
    pub dirty_count: u32,
    /// 是否绘制选中行动线（UnitActionLines 窗口内为 `true`）。
    pub action_lines_active: bool,
    /// 以 `EntityId.0` 为键的可视实体槽。
    pub units: HashMap<u64, RenderUnit>,
    /// `[AudioVisual] ConditionYellow`（0..=1）。
    pub condition_yellow: f32,
    /// `[AudioVisual] ConditionRed`（0..=1）。
    pub condition_red: f32,
    /// 本帧悬停实体（本方点选口径）。
    pub hover_id: Option<EntityId>,
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
        self.hover_id = None;
    }
}
