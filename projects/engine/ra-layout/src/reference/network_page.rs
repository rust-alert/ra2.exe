//! 网络游戏占位页 → `LayoutNode`（800×600）。

use crate::{
    geometry::Rect,
    policy::RightPanelChrome,
    reference::{from_template::shell_design_size, shell_chrome::solve_with_shell_defaults},
    snapshot::LayoutSnapshot,
    spec::{LayoutNode, fixed_rect_leaf, root_with_fixed_children},
};

/// 网络页入口 id（占位：在线不可用 + 返回）。
pub const NETWORK_BUTTON_IDS: [&str; 2] = ["online", "back"];

/// 网络占位页：居中宽钮（设计画布比例框）。
pub fn network_page_layout_tree(chrome: RightPanelChrome) -> LayoutNode {
    let children = vec![
        fixed_rect_leaf(NETWORK_BUTTON_IDS[0], Rect::from_frac(0.22, 0.36, 0.78, 0.44, chrome.shell_w, chrome.shell_h)),
        fixed_rect_leaf(NETWORK_BUTTON_IDS[1], Rect::from_frac(0.22, 0.56, 0.78, 0.64, chrome.shell_w, chrome.shell_h)),
    ];
    root_with_fixed_children("network", shell_design_size(chrome), children)
}

/// 用壳层默认 chrome 求解 [`network_page_layout_tree`]。
pub fn solve_network_page() -> LayoutSnapshot {
    solve_with_shell_defaults(network_page_layout_tree)
}
