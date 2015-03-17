//! 右栏 chrome 按钮列 → `LayoutNode`（无页面专名）。

use crate::{
    geometry::Rect,
    policy::{bottom_cover_button, RightPanelChrome},
    reference::from_template::shell_design_size,
    spec::{fixed_rect_leaf, root_with_fixed_children, LayoutNode},
};

/// 右栏连续平铺格 + 可选贴底盖按钮。
///
/// `stacked_ids` 从 tile 0 起依次占格；`bottom_id` 若有则贴底盖上沿。
pub fn right_rail_buttons_layout_tree(
    root_id: impl Into<String>,
    stacked_ids: &[&str],
    bottom_id: Option<&str>,
    chrome: RightPanelChrome,
) -> LayoutNode {
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
    root_with_fixed_children(root_id, shell_design_size(chrome), children)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{LayoutEngine, Viewport};

    #[test]
    fn main_menu_style_stack_matches_ui_layout_buttons() {
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
        let legacy = crate::ui_layout::main_menu_layout(800, 600);
        for (i, id) in [
            "single_player",
            "ww_online",
            "network",
            "movies",
            "options",
            "exit",
        ]
        .iter()
        .enumerate()
        {
            let cell = legacy.buttons[i];
            let got = snap.get(id).expect(id).layout.rect;
            assert_eq!(got.x as i32, cell.x, "{id} x");
            assert_eq!(got.y as i32, cell.y, "{id} y");
            assert_eq!(got.width as i32, cell.w, "{id} w");
            assert_eq!(got.height as i32, cell.h, "{id} h");
        }
    }
}
