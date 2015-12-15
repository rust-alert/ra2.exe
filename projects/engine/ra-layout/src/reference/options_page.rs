//! 选项页左侧内容板固定设计矩形 → `LayoutNode`。

use crate::{
    geometry::Rect,
    policy::RightPanelChrome,
    reference::from_template::shell_design_size,
    reference::shell_chrome::{
        right_rail_button_children, shell_chrome_children, solve_with_shell_defaults,
    },
    shell::LOWER_STRIP_H,
    snapshot::LayoutSnapshot,
    spec::{fixed_rect_leaf, root_with_fixed_children, LayoutNode},
};

fn rect_i(x: i32, y: i32, w: i32, h: i32) -> Rect {
    Rect::from_xywh(x as f32, y as f32, w as f32, h as f32)
}

/// 选项页右栏钮（与 `OPTIONS_BUTTON_IDS` 同序：接受 / 取消 / 主菜单贴底）。
const OPTIONS_RAIL_STACKED: &[&str] = &["accept", "cancel"];
const OPTIONS_RAIL_BOTTOM: &str = "main_menu";

/// 选项页内容控件 id（与 `solve_options_page` snapshot 对齐）。
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

/// 分区标题高。
const SEC_H: i32 = 18;
/// 滑条高。
const TRACK_H: i32 = 22;
/// 分辨率面高。
const RES_H: i32 = 28;
/// 勾选行高。
const CHECK_H: i32 = 22;
/// 分区顶 → 滑条顶：留给分区字 + 分隔线 + 滑条标签。
const SEC_TO_TRACK: i32 = 40;
/// 相邻滑条顶距（中间再插一行标签）。
const TRACK_STACK: i32 = 40;
/// 滑条底 → 下一分区顶。
const TRACK_TO_SEC: i32 = 16;
/// 勾选行距。
const CHECK_STACK: i32 = 24;

fn options_content_children(chrome: RightPanelChrome) -> Vec<LayoutNode> {
    let panel_x = chrome.panel_x() as i32;
    let content_x = 16;
    let content_y = 16;
    let content_w = panel_x - 24;
    // 底边停在金属底条之上，避免底板盖住 `lwscrnl`。
    let content_bottom = chrome.shell_h as i32 - LOWER_STRIP_H - 4;
    let content_h = (content_bottom - content_y).max(1);
    let left = content_x + 16;
    let usable_w = content_w - 32;
    let col_w = usable_w / 2 - 8;
    let right_col = left + col_w + 16;

    let mut y = content_y + 12;
    let mut nodes = vec![fixed_rect_leaf(
        "content",
        rect_i(content_x, content_y, content_w, content_h),
    )];

    nodes.push(fixed_rect_leaf("sec_display", rect_i(left, y, usable_w, SEC_H)));
    y += SEC_TO_TRACK;
    nodes.push(fixed_rect_leaf("track_detail", rect_i(left, y, col_w, TRACK_H)));
    nodes.push(fixed_rect_leaf("resolution", rect_i(right_col, y, col_w, RES_H)));
    y += TRACK_H.max(RES_H) + TRACK_TO_SEC;

    nodes.push(fixed_rect_leaf("sec_game", rect_i(left, y, usable_w, SEC_H)));
    y += SEC_TO_TRACK;
    nodes.push(fixed_rect_leaf(
        "track_difficulty",
        rect_i(left, y, usable_w - 40, TRACK_H),
    ));
    y += TRACK_H + TRACK_TO_SEC;

    nodes.push(fixed_rect_leaf("sec_ui", rect_i(left, y, usable_w, SEC_H)));
    y += SEC_TO_TRACK;
    let ui_y = y;
    nodes.push(fixed_rect_leaf("check_tooltips", rect_i(left, ui_y, 220, CHECK_H)));
    nodes.push(fixed_rect_leaf(
        "check_scanlines",
        rect_i(left, ui_y + CHECK_STACK, 220, CHECK_H),
    ));
    nodes.push(fixed_rect_leaf(
        "check_damage",
        rect_i(left, ui_y + CHECK_STACK * 2, 220, CHECK_H),
    ));
    nodes.push(fixed_rect_leaf(
        "track_scroll",
        rect_i(right_col, ui_y, col_w, TRACK_H),
    ));
    y = ui_y + CHECK_STACK * 2 + CHECK_H + TRACK_TO_SEC;

    nodes.push(fixed_rect_leaf("sec_present", rect_i(left, y, usable_w, SEC_H)));
    y += SEC_TO_TRACK;
    nodes.push(fixed_rect_leaf("check_present", rect_i(left, y, 280, CHECK_H)));
    y += CHECK_H + TRACK_TO_SEC;

    nodes.push(fixed_rect_leaf("sec_audio", rect_i(left, y, usable_w, SEC_H)));
    y += SEC_TO_TRACK;
    nodes.push(fixed_rect_leaf(
        "track_music",
        rect_i(left, y, usable_w - 40, TRACK_H),
    ));
    y += TRACK_STACK;
    nodes.push(fixed_rect_leaf(
        "track_sound",
        rect_i(left, y, usable_w - 40, TRACK_H),
    ));
    y += TRACK_STACK;
    nodes.push(fixed_rect_leaf(
        "track_voice",
        rect_i(left, y, usable_w - 40, TRACK_H),
    ));

    nodes
}

/// 选项页左侧内容板（不含右栏 chrome / 三钮）。
pub(crate) fn options_content_layout_tree(chrome: RightPanelChrome) -> LayoutNode {
    root_with_fixed_children(
        "options_content",
        shell_design_size(chrome),
        options_content_children(chrome),
    )
}

/// 选项整页：壳层 chrome + 右栏三钮 + 左侧内容板，一次求解。
pub(crate) fn options_page_layout_tree(chrome: RightPanelChrome) -> LayoutNode {
    let mut children = shell_chrome_children(chrome);
    children.extend(right_rail_button_children(
        OPTIONS_RAIL_STACKED,
        Some(OPTIONS_RAIL_BOTTOM),
        chrome,
    ));
    children.extend(options_content_children(chrome));
    root_with_fixed_children("options", shell_design_size(chrome), children)
}

/// 用壳层默认 chrome 求解 [`options_page_layout_tree`]。
pub fn solve_options_page() -> LayoutSnapshot {
    solve_with_shell_defaults(options_page_layout_tree)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{shell::rect_px_from_snapshot, LayoutEngine, Viewport};

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
        let expect = [
            ("content", 16, 16, 608, 548),
            ("sec_display", 32, 28, 576, 18),
            ("track_detail", 32, 68, 280, 22),
            ("resolution", 328, 68, 280, 28),
            ("sec_game", 32, 112, 576, 18),
            ("track_difficulty", 32, 152, 536, 22),
            ("sec_ui", 32, 190, 576, 18),
            ("check_tooltips", 32, 230, 220, 22),
            ("check_scanlines", 32, 254, 220, 22),
            ("check_damage", 32, 278, 220, 22),
            ("track_scroll", 328, 230, 280, 22),
            ("sec_present", 32, 316, 576, 18),
            ("check_present", 32, 356, 280, 22),
            ("sec_audio", 32, 394, 576, 18),
            ("track_music", 32, 434, 536, 22),
            ("track_sound", 32, 474, 536, 22),
            ("track_voice", 32, 514, 536, 22),
        ];
        for (id, x, y, w, h) in expect {
            let got = snap.get(id).expect(id).layout.rect;
            assert_eq!(got.x as i32, x, "{id} x");
            assert_eq!(got.y as i32, y, "{id} y");
            assert_eq!(got.width as i32, w, "{id} w");
            assert_eq!(got.height as i32, h, "{id} h");
        }
        assert_eq!(OPTIONS_CONTENT_IDS.len(), expect.len());
        // 内容板不得盖住底条。
        let content = rect_px_from_snapshot(&snap, "content");
        assert!(content.y + content.h <= 568, "content overlaps lower_strip");
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
        assert!(snap.get("lower_strip").is_some());
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

    #[test]
    fn options_track_labels_have_clearance_above_tracks() {
        let chrome = RightPanelChrome::shell_defaults();
        let snap = LayoutEngine.solve(
            Viewport {
                size: shell_design_size(chrome),
                ..Viewport::default()
            },
            &options_content_layout_tree(chrome),
        );
        let pairs = [
            ("sec_display", "track_detail"),
            ("sec_game", "track_difficulty"),
            ("sec_audio", "track_music"),
        ];
        for (sec, track) in pairs {
            let s = snap.get(sec).expect(sec).layout.rect;
            let t = snap.get(track).expect(track).layout.rect;
            // 分区底到滑条顶：分隔线 + 一行标签（`SEC_TO_TRACK - SEC_H` ≈ 22）。
            assert!(
                t.y >= s.y + s.height + 20.0,
                "{sec}->{track}: gap too small ({} vs {})",
                t.y,
                s.y + s.height
            );
        }
        let music = snap.get("track_music").unwrap().layout.rect;
        let sound = snap.get("track_sound").unwrap().layout.rect;
        // `TRACK_STACK - TRACK_H` = 18，刚好塞下上一行标签尾与下一行标签。
        assert!(
            sound.y >= music.y + music.height + 16.0,
            "audio tracks stacked too tight"
        );
    }
}
