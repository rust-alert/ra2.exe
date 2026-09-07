//! 帧分段计时钩子（R1）：用于区分仿真 / 快照 / 帧构建 / 提交，尚未接 GPU timestamp。

use std::time::Duration;

/// 一帧各段耗时（毫秒级诊断；未测量的段为 `None`）。
#[derive(Debug, Default, Clone)]
pub struct FrameTimings {
    /// 会话 `pump` / tick。
    pub simulation: Option<Duration>,
    /// 引擎 presentation / snapshot 构建。
    pub presentation_build: Option<Duration>,
    /// CPU 侧 frame build（更新 `RenderWorld`、填 instance）。
    pub frame_build: Option<Duration>,
    /// `queue.submit` 至完成编码。
    pub gpu_submit: Option<Duration>,
    /// present / 等待垂直同步（若可测）。
    pub present_wait: Option<Duration>,
}

impl FrameTimings {
    /// 清空各段。
    pub fn clear(&mut self) {
        *self = Self::default();
    }
}
