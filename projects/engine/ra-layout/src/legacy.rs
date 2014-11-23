// reshape-layout-components:skeleton
//! 原版页面参考（非运行时世界坐标）。

use crate::geometry::Rect;

/// 参考来源（实现侧可填字符串标签）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LegacySource {
    /// 未标注。
    Unspecified,
    /// 低分辨率对照页。
    LowResPage,
    /// 资源槽位。
    AssetSlot,
}

/// 参考在页面中的角色。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LegacyRole {
    /// 未标注。
    Unspecified,
    /// 面板。
    Panel,
    /// 按钮。
    Button,
    /// 文本。
    Text,
    /// 预览。
    Preview,
    /// 槽位。
    Slot,
}

/// 原版参考矩形与元数据。
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct LegacyReference {
    /// 参考矩形（通常为低分辨率页坐标）。
    pub rect: Rect,
    /// 来源。
    pub source: LegacySource,
    /// 角色。
    pub role: LegacyRole,
}
