//! 选项页左侧内容板固定设计矩形 → `LayoutNode`。

use crate::{
    geometry::Rect,
    policy::RightPanelChrome,
    reference::from_template::shell_design_size,
    reference::shell_chrome::{right_rail_button_children, shell_chrome_children},
    spec::{fixed_rect_leaf, root_with_fixed_children, LayoutNode},
};

fn rect_i(x: i32, y: i32, w: i32, h: i32) -> Rect {
    Rect::from_xywh(x as f32, y as f32, w as f32, h as f32)
}

/// 选项页右栏钮（与 `OPTIONS_BUTTON_IDS` 同序：接受 / 取消 / 主菜单贴底）。
const OPTIONS_RAIL_STACKED: &[&str] = &["accept", "cancel"];
const OPTIONS_RAIL_BOTTOM: &str = "main_menu";

/// 选项页内容控件 id（与过渡期 `OptionsDialogLayout` 字段同构）。
pub const OPTIONS_CONTENT_IDS: &[&str] = &[
    "content",
    "sec_display",
    "track_detail",
    "resolution",
    "sec_game",
    "track_difficulty",
    "sec_ui",
    "check_tooltips",
    "check_scanlines",
    "check_damage",
    "track_scroll",
    "sec_present",
    "check_present",
    "sec_audio",
    "track_music",
    "track_sound",
    "track_voice",
];

fn options_content_children(chrome: RightPanelChrome) -> Vec<LayoutNode> {
    let panel_x = chrome.panel_x() as i32;
    let shell_h = chrome.shell_h as i32;
    let content_x = 16;
    let content_y = 16;
    let content_w = panel_x - 24;
    let content_h = shell_h - 32;
    let left = content_x + 16;
    let usable_w = content_w - 32;
    let col_w = usable_w / 2 - 8;
    let y0 = content_y + 12;
    vec![
        fixed_rect_leaf("content", rect_i(content_x, content_y, content_w, content_h)),
        fixed_rect_leaf("sec_display", rect_i(left, y0, usable_w, 18)),
        fixed_rect_leaf("track_detail", rect_i(left, y0 + 32, col_w, 22)),
        fixed_rect_leaf(
            "resolution",
            rect_i(left + col_w + 16, y0 + 32, col_w, 28),
        ),
        fixed_rect_leaf("sec_game", rect_i(left, y0 + 80, usable_w, 18)),
        fixed_rect_leaf(
            "track_difficulty",
            rect_i(left, y0 + 112, usable_w - 40, 22),
        ),
        fixed_rect_leaf("sec_ui", rect_i(left, y0 + 168, usable_w, 18)),
        fixed_rect_leaf("check_tooltips", rect_i(left, y0 + 198, 220, 22)),
        fixed_rect_leaf("check_scanlines", rect_i(left, y0 + 224, 220, 22)),
        fixed_rect_leaf("check_damage", rect_i(left, y0 + 250, 220, 22)),
        fixed_rect_leaf(
            "track_scroll",
            rect_i(left + col_w + 16, y0 + 198, col_w, 22),
        ),
        fixed_rect_leaf("sec_present", rect_i(left, y0 + 290, usable_w, 18)),
        fixed_rect_leaf("check_present", rect_i(left, y0 + 318, 280, 22)),
        fixed_rect_leaf("sec_audio", rect_i(left, y0 + 360, usable_w, 18)),
        fixed_rect_leaf("track_music", rect_i(left, y0 + 388, usable_w - 40, 22)),
        fixed_rect_leaf("track_sound", rect_i(left, y0 + 422, usable_w - 40, 22)),
        fixed_rect_leaf("track_voice", rect_i(left, y0 + 456, usable_w - 40, 22)),
    ]
}

/// 选项页左侧内容板（不含右栏 chrome / 三钮）。
pub fn options_content_layout_tree(chrome: RightPanelChrome) -> LayoutNode {
    root_with_fixed_children(
        "options_content",
        shell_design_size(chrome),
        options_content_children(chrome),
    )
}

/// 选项整页：壳层 chrome + 右栏三钮 + 左侧内容板，一次求解。
pub fn options_page_layout_tree(chrome: RightPanelChrome) -> LayoutNode {
    let mut children = shell_chrome_children(chrome);
    children.extend(right_rail_button_children(
        OPTIONS_RAIL_STACKED,
        Some(OPTIONS_RAIL_BOTTOM),
        chrome,
    ));
    children.extend(options_content_children(chrome));
    root_with_fixed_children("options", shell_design_size(chrome), children)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{LayoutEngine, Viewport};

    #[test]
    fn options_content_matches_800x600_golden() {
        let chrome = RightPanelChrome::shell_defaults();
        let snap = LayoutEngine.solve(
            Viewport {
                size: shell_design_size(chrome),
                ..Viewport::default()
            },
            &options_content_layout_tree(chrome),
        );
        // 与过渡期 `OptionsDialogLayout::new` 在 800×600 下的内容板公式一致。
        let expect = [
            ("content", 16, 16, 608, 568),
            ("sec_display", 32, 28, 576, 18),
            ("track_detail", 32, 60, 280, 22),
            ("resolution", 328, 60, 280, 28),
            ("sec_game", 32, 108, 576, 18),
            ("track_difficulty", 32, 140, 536, 22),
            ("sec_ui", 32, 196, 576, 18),
            ("check_tooltips", 32, 226, 220, 22),
            ("check_scanlines", 32, 252, 220, 22),
            ("check_damage", 32, 278, 220, 22),
            ("track_scroll", 328, 226, 280, 22),
            ("sec_present", 32, 318, 576, 18),
            ("check_present", 32, 346, 280, 22),
            ("sec_audio", 32, 388, 576, 18),
            ("track_music", 32, 416, 536, 22),
            ("track_sound", 32, 450, 536, 22),
            ("track_voice", 32, 484, 536, 22),
        ];
        for (id, x, y, w, h) in expect {
            let got = snap.get(id).expect(id).layout.rect;
            assert_eq!(got.x as i32, x, "{id} x");
            assert_eq!(got.y as i32, y, "{id} y");
            assert_eq!(got.width as i32, w, "{id} w");
            assert_eq!(got.height as i32, h, "{id} h");
        }
        assert_eq!(OPTIONS_CONTENT_IDS.len(), expect.len());
    }

    #[test]
    fn options_page_tree_includes_rail_and_content() {
        let chrome = RightPanelChrome::shell_defaults();
        let snap = LayoutEngine.solve(
            Viewport {
                size: shell_design_size(chrome),
                ..Viewport::default()
            },
            &options_page_layout_tree(chrome),
        );
        assert!(snap.get("panel_top").is_some());
        assert!(snap.get("accept").is_some());
        assert!(snap.get("main_menu").is_some());
        assert!(snap.get("content").is_some());
        assert!(snap.get("track_voice").is_some());
        let content = options_content_layout_tree(chrome);
        let content_snap = LayoutEngine.solve(
            Viewport {
                size: shell_design_size(chrome),
                ..Viewport::default()
            },
            &content,
        );
        assert_eq!(
            snap.get("track_detail").map(|e| e.layout.rect),
            content_snap.get("track_detail").map(|e| e.layout.rect)
        );
    }
}
