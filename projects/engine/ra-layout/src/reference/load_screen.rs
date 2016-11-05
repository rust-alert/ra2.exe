//! 装载页内容区固定设计矩形 → `LayoutNode`（800×600）。

use crate::{
    geometry::Rect,
    policy::RightPanelChrome,
    reference::{from_template::shell_design_size, shell_chrome::solve_with_shell_defaults},
    snapshot::LayoutSnapshot,
    spec::{LayoutNode, fixed_rect_leaf, root_with_fixed_children},
};

/// 装载失败时操作钮 id。
pub const LOAD_SCREEN_BUTTON_IDS: [&str; 2] = ["retry", "cancel"];

pub fn rect_i(x: i32, y: i32, w: i32, h: i32) -> Rect {
    Rect::from_xywh(x as f32, y as f32, w as f32, h as f32)
}

/// 装载页：文案槽、进度条原点、地图预览区、失败时重试/取消。
pub fn load_screen_layout_tree(chrome: RightPanelChrome) -> LayoutNode {
    let children = vec![
        // 装载页文案与进度槽：800×600 设计画布固定像素。
        fixed_rect_leaf("special", rect_i(54, 106, 190, 22)),
        fixed_rect_leaf("brief", rect_i(48, 134, 340, 200)),
        fixed_rect_leaf("name", rect_i(648, 538, 120, 24)),
        fixed_rect_leaf("status", rect_i(56, 310, 160, 20)),
        fixed_rect_leaf("progress", rect_i(56, 332, 200, 16)),
        fixed_rect_leaf("player_flag", rect_i(150, 324, 40, 24)),
        fixed_rect_leaf("player_name", rect_i(202, 328, 120, 20)),
        // 选中图预览区：对齐原生 `mmpb` 800 分支 (499,379,216,166)。
        fixed_rect_leaf("map_preview", rect_i(499, 379, 216, 166)),
        fixed_rect_leaf(LOAD_SCREEN_BUTTON_IDS[0], Rect::from_frac(0.30, 0.88, 0.50, 0.96, chrome.shell_w, chrome.shell_h)),
        fixed_rect_leaf(LOAD_SCREEN_BUTTON_IDS[1], Rect::from_frac(0.54, 0.88, 0.74, 0.96, chrome.shell_w, chrome.shell_h)),
    ];
    root_with_fixed_children("load_screen", shell_design_size(chrome), children)
}

/// 用壳层默认 chrome 求解 [`load_screen_layout_tree`]。
pub fn solve_load_screen() -> LayoutSnapshot {
    solve_with_shell_defaults(load_screen_layout_tree)
}
