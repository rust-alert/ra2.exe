//! 右栏 chrome 按钮列 → `LayoutNode`（无页面专名）。

use crate::{
    geometry::Rect,
    policy::{RightPanelChrome, bottom_cover_button},
    reference::from_template::shell_design_size,
    snapshot::LayoutSnapshot,
    solver::LayoutEngine,
    spec::{LayoutNode, fixed_rect_leaf, root_with_fixed_children},
    viewport::Viewport,
};

/// 壳层底装饰条高（`lwscrnl`）。
pub const LOWER_STRIP_H: f32 = 32.0;
/// 影片 / 背景区高（原版 `ra2ts_l` 可视高）。
pub const MOVIE_H: f32 = 570.0;
/// 右栏顶盖内页标题宽。
pub const TITLE_W: f32 = 163.0;
pub const TITLE_H: f32 = 18.0;
pub const TITLE_INSET_X: f32 = 3.0;
pub const TITLE_Y: f32 = 9.0;
/// 左下角悬停提示行。
pub const TOOLTIP_X: f32 = 10.0;
pub const TOOLTIP_W: f32 = 455.0;
pub const TOOLTIP_H: f32 = 20.0;
pub const TOOLTIP_BOTTOM_GAP: f32 = 1.0;

pub fn shell_chrome_children(chrome: RightPanelChrome) -> Vec<LayoutNode> {
    let mut children = shell_panel_chrome_children(chrome);
    let panel_x = chrome.panel_x();
    children.push(fixed_rect_leaf("lower_strip", Rect::from_xywh(0.0, chrome.shell_h - LOWER_STRIP_H, panel_x, LOWER_STRIP_H)));
    children.push(fixed_rect_leaf("title", Rect::from_xywh(panel_x + TITLE_INSET_X, TITLE_Y, TITLE_W, TITLE_H)));
    children
        .push(fixed_rect_leaf("tooltip", Rect::from_xywh(TOOLTIP_X, chrome.shell_h - TOOLTIP_H - TOOLTIP_BOTTOM_GAP, TOOLTIP_W, TOOLTIP_H)));
    children
}

/// 对话框页共用的面板 / 影片区 / 底装饰条（不含标题、提示，避免与模板控件 id 冲突）。
pub fn shell_panel_chrome_children(chrome: RightPanelChrome) -> Vec<LayoutNode> {
    let panel_x = chrome.panel_x();
    let movie_w = panel_x;
    let bottom_y = chrome.panel_bottom_y();
    vec![
        fixed_rect_leaf("panel_top", Rect::from_xywh(panel_x, 0.0, chrome.panel_w, chrome.panel_top_h)),
        fixed_rect_leaf("panel_tile", Rect::from_xywh(panel_x, chrome.tile_y(), chrome.panel_w, chrome.tile_h)),
        fixed_rect_leaf("panel_bottom", Rect::from_xywh(panel_x, bottom_y, chrome.panel_w, chrome.shell_h - bottom_y)),
        fixed_rect_leaf("background", Rect::from_xywh(0.0, 0.0, movie_w, MOVIE_H)),
        fixed_rect_leaf("movie", Rect::from_xywh(0.0, 0.0, movie_w, MOVIE_H)),
        // 与壳层菜单共用 `lwscrnl` 底条；提示文案由模板 `status_help`（ShellTooltip）承载。
        fixed_rect_leaf("lower_strip", Rect::from_xywh(0.0, chrome.shell_h - LOWER_STRIP_H, panel_x, LOWER_STRIP_H)),
    ]
}

pub fn right_rail_button_children(stacked_ids: &[&str], bottom_id: Option<&str>, chrome: RightPanelChrome) -> Vec<LayoutNode> {
    let mut children = Vec::with_capacity(stacked_ids.len() + bottom_id.is_some() as usize);
    for (i, id) in stacked_ids.iter().enumerate() {
        let rect = Rect::from_xywh(chrome.button_x(), chrome.tile_y() + i as f32 * chrome.tile_h, chrome.button_w, chrome.button_h);
        children.push(fixed_rect_leaf(*id, rect));
    }
    if let Some(id) = bottom_id {
        children.push(fixed_rect_leaf(id, bottom_cover_button(chrome)));
    }
    children
}

/// 壳层共享 chrome（面板条带、影片区、标题、提示），不含页面按钮。
pub fn shell_chrome_layout_tree(chrome: RightPanelChrome) -> LayoutNode {
    root_with_fixed_children("shell_chrome", shell_design_size(chrome), shell_chrome_children(chrome))
}

/// 右栏连续平铺格 + 可选贴底盖按钮。
///
/// `stacked_ids` 从 tile 0 起依次占格；`bottom_id` 若有则贴底盖上沿。
pub fn right_rail_buttons_layout_tree(
    root_id: impl Into<String>,
    stacked_ids: &[&str],
    bottom_id: Option<&str>,
    chrome: RightPanelChrome,
) -> LayoutNode {
    root_with_fixed_children(root_id, shell_design_size(chrome), right_rail_button_children(stacked_ids, bottom_id, chrome))
}

/// 壳层页面：共享 chrome + 右栏按钮，一次求解。
pub fn shell_page_layout_tree(
    root_id: impl Into<String>,
    stacked_ids: &[&str],
    bottom_id: Option<&str>,
    chrome: RightPanelChrome,
) -> LayoutNode {
    let mut children = shell_chrome_children(chrome);
    children.extend(right_rail_button_children(stacked_ids, bottom_id, chrome));
    root_with_fixed_children(root_id, shell_design_size(chrome), children)
}

/// 用壳层默认 chrome 求解任意布局树（shell 设计尺寸 viewport）。
pub fn solve_with_shell_defaults(build: impl FnOnce(RightPanelChrome) -> LayoutNode) -> LayoutSnapshot {
    let chrome = RightPanelChrome::shell_defaults();
    LayoutEngine.solve(Viewport { size: shell_design_size(chrome), ..Viewport::default() }, &build(chrome))
}

/// 用壳层默认 chrome 求解 [`shell_page_layout_tree`]（hit / compose 共用）。
pub fn solve_shell_page(root_id: impl Into<String>, stacked_ids: &[&str], bottom_id: Option<&str>) -> LayoutSnapshot {
    let root_id = root_id.into();
    solve_with_shell_defaults(|chrome| shell_page_layout_tree(root_id, stacked_ids, bottom_id, chrome))
}
