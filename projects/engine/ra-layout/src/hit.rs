// reshape-layout-components:skeleton
//! 命中区域（与 LayoutBox 同源）。

use crate::geometry::Rect;

/// 命中模式。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum HitTestMode {
    /// 不参与命中。
    #[default]
    None,
    /// 矩形命中。
    Rect,
}

/// 命中区域。
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct HitRegion {
    /// 与视觉矩形一致时直接复用。
    pub rect: Rect,
}
