//! 装载页内容区固定设计矩形 → `LayoutNode`（800×600）。

use crate::{
    geometry::Rect,
    policy::RightPanelChrome,
    reference::from_template::shell_design_size,
    reference::shell_chrome::solve_with_shell_defaults,
    snapshot::LayoutSnapshot,
    spec::{fixed_rect_leaf, root_with_fixed_children, LayoutNode},
};

/// 装载失败时操作钮 id。
pub const LOAD_SCREEN_BUTTON_IDS: [&str; 2] = ["retry", "cancel"];

fn rect_i(x: i32, y: i32, w: i32, h: i32) -> Rect {
    Rect::from_xywh(x as f32, y as f32, w as f32, h as f32)
}

/// 装载页：文案槽、进度条原点、失败时重试/取消。
pub fn load_screen_layout_tree(chrome: RightPanelChrome) -> LayoutNode {
    let children = vec![
        // 对照原版截图的 800×600 像素映射（与过渡期 compose 常量一致）。
        fixed_rect_leaf("special", rect_i(54, 106, 190, 22)),
        fixed_rect_leaf("brief", rect_i(48, 134, 340, 200)),
        fixed_rect_leaf("name", rect_i(648, 538, 120, 24)),
        fixed_rect_leaf("status", rect_i(56, 310, 160, 20)),
        fixed_rect_leaf("progress", rect_i(56, 332, 200, 16)),
        fixed_rect_leaf("player_flag", rect_i(150, 324, 40, 24)),
        fixed_rect_leaf("player_name", rect_i(202, 328, 120, 20)),
        fixed_rect_leaf(
            LOAD_SCREEN_BUTTON_IDS[0],
            Rect::from_frac(0.30, 0.88, 0.50, 0.96, chrome.shell_w, chrome.shell_h),
        ),
        fixed_rect_leaf(
            LOAD_SCREEN_BUTTON_IDS[1],
            Rect::from_frac(0.54, 0.88, 0.74, 0.96, chrome.shell_w, chrome.shell_h),
        ),
    ];
    root_with_fixed_children("load_screen", shell_design_size(chrome), children)
}

/// 用壳层默认 chrome 求解 [`load_screen_layout_tree`]。
pub fn solve_load_screen() -> LayoutSnapshot {
    solve_with_shell_defaults(load_screen_layout_tree)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{LayoutEngine, Size2, Viewport};

    #[test]
    fn load_screen_tree_matches_compose_constants() {
        let chrome = RightPanelChrome::shell_defaults();
        let snap = LayoutEngine.solve(
            Viewport {
                size: Size2 {
                    width: chrome.shell_w,
                    height: chrome.shell_h,
                },
                ..Viewport::default()
            },
            &load_screen_layout_tree(chrome),
        );
        for (id, x, y, w, h) in [
            ("special", 54, 106, 190, 22),
            ("brief", 48, 134, 340, 200),
            ("name", 648, 538, 120, 24),
            ("status", 56, 310, 160, 20),
            ("progress", 56, 332, 200, 16),
            ("player_flag", 150, 324, 40, 24),
            ("player_name", 202, 328, 120, 20),
        ] {
            let got = snap.get(id).expect(id).layout.rect;
            assert_eq!(got.x as i32, x, "{id} x");
            assert_eq!(got.y as i32, y, "{id} y");
            assert_eq!(got.width as i32, w, "{id} w");
            assert_eq!(got.height as i32, h, "{id} h");
        }
        let retry = snap.get("retry").unwrap().layout.rect;
        assert_eq!(retry.x as i32, 240);
        assert_eq!(retry.y as i32, 528);
        assert_eq!(retry.width as i32, 160);
        assert_eq!(retry.height as i32, 48);
        let cancel = snap.get("cancel").unwrap().layout.rect;
        assert_eq!(cancel.x as i32, 432);
        assert_eq!(cancel.y as i32, 528);
    }
}
