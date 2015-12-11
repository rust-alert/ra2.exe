//! 右栏 chrome 按钮列 → `LayoutNode`（无页面专名）。

use crate::{
    geometry::Rect,
    policy::{bottom_cover_button, RightPanelChrome},
    reference::from_template::shell_design_size,
    snapshot::LayoutSnapshot,
    solver::LayoutEngine,
    spec::{fixed_rect_leaf, root_with_fixed_children, LayoutNode},
    viewport::Viewport,
};

/// 壳层底装饰条高（`lwscrnl`）。
const LOWER_STRIP_H: f32 = 32.0;
/// 影片 / 背景区高（原版 `ra2ts_l` 可视高）。
const MOVIE_H: f32 = 570.0;
/// 右栏顶盖内页标题宽。
const TITLE_W: f32 = 163.0;
const TITLE_H: f32 = 18.0;
const TITLE_INSET_X: f32 = 3.0;
const TITLE_Y: f32 = 9.0;
/// 左下角悬停提示行。
const TOOLTIP_X: f32 = 10.0;
const TOOLTIP_W: f32 = 455.0;
const TOOLTIP_H: f32 = 20.0;
const TOOLTIP_BOTTOM_GAP: f32 = 1.0;

pub(crate) fn shell_chrome_children(chrome: RightPanelChrome) -> Vec<LayoutNode> {
    let mut children = shell_panel_chrome_children(chrome);
    let panel_x = chrome.panel_x();
    children.push(fixed_rect_leaf(
        "lower_strip",
        Rect::from_xywh(0.0, chrome.shell_h - LOWER_STRIP_H, panel_x, LOWER_STRIP_H),
    ));
    children.push(fixed_rect_leaf(
        "title",
        Rect::from_xywh(panel_x + TITLE_INSET_X, TITLE_Y, TITLE_W, TITLE_H),
    ));
    children.push(fixed_rect_leaf(
        "tooltip",
        Rect::from_xywh(
            TOOLTIP_X,
            chrome.shell_h - TOOLTIP_H - TOOLTIP_BOTTOM_GAP,
            TOOLTIP_W,
            TOOLTIP_H,
        ),
    ));
    children
}

/// 对话框页共用的面板 / 影片区 / 底装饰条（不含标题、提示，避免与模板控件 id 冲突）。
pub(crate) fn shell_panel_chrome_children(chrome: RightPanelChrome) -> Vec<LayoutNode> {
    let panel_x = chrome.panel_x();
    let movie_w = panel_x;
    let bottom_y = chrome.panel_bottom_y();
    vec![
        fixed_rect_leaf(
            "panel_top",
            Rect::from_xywh(panel_x, 0.0, chrome.panel_w, chrome.panel_top_h),
        ),
        fixed_rect_leaf(
            "panel_tile",
            Rect::from_xywh(panel_x, chrome.tile_y(), chrome.panel_w, chrome.tile_h),
        ),
        fixed_rect_leaf(
            "panel_bottom",
            Rect::from_xywh(panel_x, bottom_y, chrome.panel_w, chrome.shell_h - bottom_y),
        ),
        fixed_rect_leaf(
            "background",
            Rect::from_xywh(0.0, 0.0, movie_w, MOVIE_H),
        ),
        fixed_rect_leaf("movie", Rect::from_xywh(0.0, 0.0, movie_w, MOVIE_H)),
        // 与壳层菜单共用 `lwscrnl` 底条；提示文案由模板 `status_help`（ShellTooltip）承载。
        fixed_rect_leaf(
            "lower_strip",
            Rect::from_xywh(0.0, chrome.shell_h - LOWER_STRIP_H, panel_x, LOWER_STRIP_H),
        ),
    ]
}

pub(crate) fn right_rail_button_children(
    stacked_ids: &[&str],
    bottom_id: Option<&str>,
    chrome: RightPanelChrome,
) -> Vec<LayoutNode> {
    let mut children = Vec::with_capacity(stacked_ids.len() + bottom_id.is_some() as usize);
    for (i, id) in stacked_ids.iter().enumerate() {
        let rect = Rect::from_xywh(
            chrome.button_x(),
            chrome.tile_y() + i as f32 * chrome.tile_h,
            chrome.button_w,
            chrome.button_h,
        );
        children.push(fixed_rect_leaf(*id, rect));
    }
    if let Some(id) = bottom_id {
        children.push(fixed_rect_leaf(id, bottom_cover_button(chrome)));
    }
    children
}

/// 壳层共享 chrome（面板条带、影片区、标题、提示），不含页面按钮。
pub(crate) fn shell_chrome_layout_tree(chrome: RightPanelChrome) -> LayoutNode {
    root_with_fixed_children(
        "shell_chrome",
        shell_design_size(chrome),
        shell_chrome_children(chrome),
    )
}

/// 右栏连续平铺格 + 可选贴底盖按钮。
///
/// `stacked_ids` 从 tile 0 起依次占格；`bottom_id` 若有则贴底盖上沿。
pub(crate) fn right_rail_buttons_layout_tree(
    root_id: impl Into<String>,
    stacked_ids: &[&str],
    bottom_id: Option<&str>,
    chrome: RightPanelChrome,
) -> LayoutNode {
    root_with_fixed_children(
        root_id,
        shell_design_size(chrome),
        right_rail_button_children(stacked_ids, bottom_id, chrome),
    )
}

/// 壳层页面：共享 chrome + 右栏按钮，一次求解。
pub(crate) fn shell_page_layout_tree(
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
pub(crate) fn solve_with_shell_defaults(
    build: impl FnOnce(RightPanelChrome) -> LayoutNode,
) -> LayoutSnapshot {
    let chrome = RightPanelChrome::shell_defaults();
    LayoutEngine.solve(
        Viewport {
            size: shell_design_size(chrome),
            ..Viewport::default()
        },
        &build(chrome),
    )
}

/// 用壳层默认 chrome 求解 [`shell_page_layout_tree`]（hit / compose 共用）。
pub fn solve_shell_page(
    root_id: impl Into<String>,
    stacked_ids: &[&str],
    bottom_id: Option<&str>,
) -> LayoutSnapshot {
    let root_id = root_id.into();
    solve_with_shell_defaults(|chrome| {
        shell_page_layout_tree(root_id, stacked_ids, bottom_id, chrome)
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        shell::{rect_px_from_snapshot, RectPx},
        LayoutEngine, Viewport,
    };

    #[test]
    fn shell_chrome_matches_golden_slots() {
        let chrome = RightPanelChrome::shell_defaults();
        let snap = LayoutEngine.solve(
            Viewport {
                size: shell_design_size(chrome),
                ..Viewport::default()
            },
            &shell_chrome_layout_tree(chrome),
        );
        assert_eq!(
            rect_px_from_snapshot(&snap, "panel_top"),
            RectPx::new(632, 0, 168, 199)
        );
        assert_eq!(
            rect_px_from_snapshot(&snap, "panel_tile"),
            RectPx::new(632, 199, 168, 42)
        );
        assert_eq!(
            rect_px_from_snapshot(&snap, "panel_bottom"),
            RectPx::new(632, 577, 168, 23)
        );
        assert_eq!(
            rect_px_from_snapshot(&snap, "background"),
            RectPx::new(0, 0, 632, 570)
        );
        assert_eq!(
            rect_px_from_snapshot(&snap, "movie"),
            RectPx::new(0, 0, 632, 570)
        );
        assert_eq!(
            rect_px_from_snapshot(&snap, "lower_strip"),
            RectPx::new(0, 568, 632, 32)
        );
        assert_eq!(
            rect_px_from_snapshot(&snap, "title"),
            RectPx::new(635, 9, 163, 18)
        );
        assert_eq!(
            rect_px_from_snapshot(&snap, "tooltip"),
            RectPx::new(10, 579, 455, 20)
        );
        assert_eq!(chrome.tile_count(), 9);
    }

    #[test]
    fn main_menu_rail_buttons_match_golden_cells() {
        let chrome = RightPanelChrome::shell_defaults();
        let root = right_rail_buttons_layout_tree(
            "main_menu",
            &["single_player", "ww_online", "network", "movies", "options"],
            Some("exit"),
            chrome,
        );
        let snap = LayoutEngine.solve(
            Viewport {
                size: shell_design_size(chrome),
                ..Viewport::default()
            },
            &root,
        );
        let expected = [
            ("single_player", RectPx::new(644, 199, 156, 42)),
            ("ww_online", RectPx::new(644, 241, 156, 42)),
            ("network", RectPx::new(644, 283, 156, 42)),
            ("movies", RectPx::new(644, 325, 156, 42)),
            ("options", RectPx::new(644, 367, 156, 42)),
            ("exit", RectPx::new(644, 535, 156, 42)),
        ];
        for (id, cell) in expected {
            assert_eq!(rect_px_from_snapshot(&snap, id), cell, "{id}");
        }
    }

    #[test]
    fn shell_page_tree_matches_chrome_and_buttons() {
        let chrome = RightPanelChrome::shell_defaults();
        let ids = [
            "single_player",
            "ww_online",
            "network",
            "movies",
            "options",
            "exit",
        ];
        let snap = LayoutEngine.solve(
            Viewport {
                size: shell_design_size(chrome),
                ..Viewport::default()
            },
            &shell_page_layout_tree("main_menu", &ids[..5], Some(ids[5]), chrome),
        );
        assert_eq!(
            rect_px_from_snapshot(&snap, "title"),
            RectPx::new(635, 9, 163, 18)
        );
        assert_eq!(
            rect_px_from_snapshot(&snap, "exit"),
            RectPx::new(644, 535, 156, 42)
        );
    }
}
