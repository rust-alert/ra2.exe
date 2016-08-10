//! 右栏水平锚定。

use crate::{geometry::Rect, policy::tile_snap::RightPanelChrome};

/// 相对右栏宽度水平居中，并锚到右缘。
pub fn right_panel_anchor(base: Rect, chrome: RightPanelChrome) -> Rect {
    let inset = ((chrome.panel_w - base.width) * 0.5).floor();
    Rect::from_xywh(chrome.shell_w - base.width - inset, base.y, base.width, base.height)
}
