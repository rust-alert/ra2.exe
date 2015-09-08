//! 网络游戏占位页 → `LayoutNode`（800×600）。

use crate::{
    geometry::Rect,
    policy::RightPanelChrome,
    reference::from_template::shell_design_size,
    reference::shell_chrome::solve_with_shell_defaults,
    snapshot::LayoutSnapshot,
    spec::{fixed_rect_leaf, root_with_fixed_children, LayoutNode},
};

/// 网络页入口 id（占位：在线不可用 + 返回）。
pub const NETWORK_BUTTON_IDS: [&str; 2] = ["online", "back"];

/// 归一化命中框 → 800×600 设计像素。
fn frac_rect(x0: f32, y0: f32, x1: f32, y1: f32, chrome: RightPanelChrome) -> Rect {
    let w = chrome.shell_w;
    let h = chrome.shell_h;
    Rect::from_xywh(
        (x0 * w).round(),
        (y0 * h).round(),
        ((x1 - x0) * w).round().max(1.0),
        ((y1 - y0) * h).round().max(1.0),
    )
}

/// 网络占位页：居中宽钮（与过渡期 `NETWORK_BUTTONS` 归一化框同构）。
pub fn network_page_layout_tree(chrome: RightPanelChrome) -> LayoutNode {
    let children = vec![
        fixed_rect_leaf(
            NETWORK_BUTTON_IDS[0],
            frac_rect(0.22, 0.36, 0.78, 0.44, chrome),
        ),
        fixed_rect_leaf(
            NETWORK_BUTTON_IDS[1],
            frac_rect(0.22, 0.56, 0.78, 0.64, chrome),
        ),
    ];
    root_with_fixed_children("network", shell_design_size(chrome), children)
}

/// 用壳层默认 chrome 求解 [`network_page_layout_tree`]。
pub fn solve_network_page() -> LayoutSnapshot {
    solve_with_shell_defaults(network_page_layout_tree)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn network_page_matches_legacy_slot_fractions() {
        let snap = solve_network_page();
        let online = snap.get("online").unwrap().layout.rect;
        assert_eq!(online.x as i32, 176);
        assert_eq!(online.y as i32, 216);
        assert_eq!(online.width as i32, 448);
        assert_eq!(online.height as i32, 48);
        let back = snap.get("back").unwrap().layout.rect;
        assert_eq!(back.x as i32, 176);
        assert_eq!(back.y as i32, 336);
        assert_eq!(back.width as i32, 448);
        assert_eq!(back.height as i32, 48);
    }
}
