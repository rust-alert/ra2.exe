//! 可跨帧复用的渲染世界（R1 骨架）。
//!
//! 由 [`crate::frame::FrameBuilder`] 根据引擎只读投影 / 事件增量更新。
//! 不持有权威 `MatchState`，不在此解码 MIX/INI。

/// 当前可视对象的渲染侧状态（跨帧复用，避免每显示帧全量重建）。
#[derive(Debug, Default)]
pub struct RenderWorld {
    /// 逻辑仿真 tick（与引擎投影对齐；插值用）。
    pub source_tick: u64,
    /// 本帧脏实体数（诊断；完整脏集在后续增量阶段落地）。
    pub dirty_count: u32,
}
