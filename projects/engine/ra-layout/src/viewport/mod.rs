//! 视口与 DPI。

use crate::geometry::{Insets, Size2};

/// 当前宿主可用区域。
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Viewport {
    /// 逻辑宽高。
    pub size: Size2,
    /// 设备像素比。
    pub device_scale_factor: f32,
    /// 安全区。
    pub safe_area: Insets,
}

impl Default for Viewport {
    fn default() -> Self {
        Self { size: Size2 { width: 1280.0, height: 720.0 }, device_scale_factor: 1.0, safe_area: Insets::default() }
    }
}
