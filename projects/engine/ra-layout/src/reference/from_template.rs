//! 由 `DialogTemplate` + chrome 策略解析设计像素。

use ra_types::{
    dialog_template_0x102, dialog_template_0x6b, ControlPlacement, DialogControlDesc, DialogTemplate,
};

use crate::{
    geometry::{Rect, Size2},
    policy::{bottom_cover_button, right_panel_anchor, tile_snap_button, RightPanelChrome},
    reference::{
        shell_chrome::{shell_panel_chrome_children, solve_with_shell_defaults},
        DluRect, MS_SANS_SERIF_8PT,
    },
    snapshot::LayoutSnapshot,
    spec::{fixed_rect_leaf, root_with_fixed_children, LayoutNode},
};

fn dlu_of(desc: &DialogControlDesc) -> Rect {
    DluRect::new(desc.dlu_x, desc.dlu_y, desc.dlu_w, desc.dlu_h).to_design_px(MS_SANS_SERIF_8PT)
}

fn map_name_plate(chrome: RightPanelChrome) -> Rect {
    const PLATE_W: f32 = 156.0;
    const PLATE_H: f32 = 84.0;
    Rect::from_xywh(
        chrome.shell_w - PLATE_W,
        chrome.panel_top_h + chrome.tile_h - PLATE_H,
        PLATE_W,
        PLATE_H,
    )
}

/// 壳层底栏提示带（与 `shell_chrome` tooltip 同几何）。
fn shell_tooltip_rect(chrome: RightPanelChrome, desc: &DialogControlDesc) -> Rect {
    const TOOLTIP_H: f32 = 20.0;
    const TOOLTIP_BOTTOM_GAP: f32 = 1.0;
    let base = dlu_of(desc);
    let max_w = (chrome.panel_x() - base.x).max(1.0);
    Rect::from_xywh(
        base.x,
        chrome.shell_h - TOOLTIP_H - TOOLTIP_BOTTOM_GAP,
        base.width.min(max_w),
        TOOLTIP_H,
    )
}

/// 解析单个控件描述。
pub(crate) fn resolve_control_desc(desc: &DialogControlDesc, chrome: RightPanelChrome) -> Rect {
    match desc.placement {
        ControlPlacement::PreserveDlu => dlu_of(desc),
        ControlPlacement::TileSnap => tile_snap_button(dlu_of(desc), chrome),
        ControlPlacement::RightPanelAnchor => right_panel_anchor(dlu_of(desc), chrome),
        ControlPlacement::BottomCoverButton => bottom_cover_button(chrome),
        ControlPlacement::MapNamePlate => map_name_plate(chrome),
        ControlPlacement::ComboFace => {
            let mut rect = dlu_of(desc);
            // 与壳层 `SKIRMISH_COMBO_FACE_H` 一致。
            rect.height = 24.0;
            rect
        }
        ControlPlacement::ShellTooltip => shell_tooltip_rect(chrome, desc),
    }
}

/// 壳层设计视口。
pub(crate) fn shell_design_size(chrome: RightPanelChrome) -> Size2 {
    Size2 {
        width: chrome.shell_w,
        height: chrome.shell_h,
    }
}

/// 对话框整页：面板 chrome（含底条，不含标题 / 提示 id）+ 模板控件，一次求解。
pub(crate) fn dialog_page_layout_tree(
    root_id: impl Into<String>,
    template: &DialogTemplate,
    chrome: RightPanelChrome,
) -> LayoutNode {
    dialog_page_layout_tree_ex(root_id, template, chrome, false)
}

/// 同 [`dialog_page_layout_tree`]；`center_left_form` 时把左栏表单块在内容区居中。
pub(crate) fn dialog_page_layout_tree_ex(
    root_id: impl Into<String>,
    template: &DialogTemplate,
    chrome: RightPanelChrome,
    center_left_form: bool,
) -> LayoutNode {
    let mut children = shell_panel_chrome_children(chrome);
    children.extend(dialog_control_children(template, chrome, center_left_form));
    root_with_fixed_children(root_id, shell_design_size(chrome), children)
}

fn is_left_content_placement(placement: ControlPlacement) -> bool {
    matches!(
        placement,
        ControlPlacement::PreserveDlu | ControlPlacement::ComboFace
    )
}

/// 将左栏表单块在内容区（右栏左侧）水平+垂直居中，避免贴左上角。
fn center_rects_in_content_area(rects: &mut [Rect], chrome: RightPanelChrome) {
    if rects.is_empty() {
        return;
    }
    const MARGIN: f32 = 28.0;
    const BOTTOM_RESERVE: f32 = 48.0;
    let panel_x = chrome.panel_x();
    let mut min_x = f32::INFINITY;
    let mut min_y = f32::INFINITY;
    let mut max_r = f32::NEG_INFINITY;
    let mut max_b = f32::NEG_INFINITY;
    for r in rects.iter() {
        min_x = min_x.min(r.x);
        min_y = min_y.min(r.y);
        max_r = max_r.max(r.x + r.width);
        max_b = max_b.max(r.y + r.height);
    }
    let block_w = (max_r - min_x).max(0.0);
    let block_h = (max_b - min_y).max(0.0);
    let avail_w = (panel_x - MARGIN * 2.0).max(0.0);
    let avail_h = (chrome.shell_h - BOTTOM_RESERVE - MARGIN * 2.0).max(0.0);
    let offset_x = MARGIN + (avail_w - block_w).max(0.0) * 0.5 - min_x;
    let offset_y = MARGIN + (avail_h - block_h).max(0.0) * 0.5 - min_y;
    for r in rects.iter_mut() {
        r.x += offset_x;
        r.y += offset_y;
    }
}

fn dialog_control_children(
    template: &DialogTemplate,
    chrome: RightPanelChrome,
    center_left_form: bool,
) -> Vec<LayoutNode> {
    let mut left: Vec<(String, Rect)> = Vec::new();
    let mut other: Vec<(String, Rect)> = Vec::new();
    let mut controls: Vec<&DialogControlDesc> = template.controls.iter().collect();
    controls.sort_by_key(|c| match c.placement {
        ControlPlacement::TileSnap | ControlPlacement::BottomCoverButton => 1_u8,
        _ => 0,
    });
    for c in controls {
        let rect = resolve_control_desc(c, chrome);
        if is_left_content_placement(c.placement) {
            left.push((c.id.0.clone(), rect));
        } else {
            other.push((c.id.0.clone(), rect));
        }
    }
    if center_left_form {
        let mut left_rects: Vec<Rect> = left.iter().map(|(_, r)| *r).collect();
        center_rects_in_content_area(&mut left_rects, chrome);
        for (i, r) in left_rects.into_iter().enumerate() {
            left[i].1 = r;
        }
    }

    left.into_iter()
        .chain(other)
        .map(|(id, rect)| fixed_rect_leaf(id, rect))
        .collect()
}

/// 选图页：面板 chrome + `0x6B` 模板 → 左栏列表在内容区居中。
pub fn solve_choose_map() -> LayoutSnapshot {
    solve_with_shell_defaults(|chrome| {
        dialog_page_layout_tree_ex("dialog_0x6b", &dialog_template_0x6b(), chrome, true)
    })
}

/// 遭遇战大厅：面板 chrome + `0x102` 模板 → 左栏表单在内容区居中。
pub fn solve_skirmish_lobby() -> LayoutSnapshot {
    solve_with_shell_defaults(|chrome| {
        dialog_page_layout_tree_ex("dialog_0x102", &dialog_template_0x102(), chrome, true)
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::shell::{rect_px_from_snapshot, RectPx};

    #[test]
    fn skirmish_lobby_has_lower_strip_and_bottom_status() {
        let snap = solve_skirmish_lobby();
        assert_eq!(
            rect_px_from_snapshot(&snap, "lower_strip"),
            RectPx::new(0, 568, 632, 32)
        );
        assert_eq!(
            rect_px_from_snapshot(&snap, "status_help"),
            RectPx::new(15, 579, 455, 20)
        );
        assert_eq!(rect_px_from_snapshot(&snap, "player_name").h, 24);
        assert_eq!(rect_px_from_snapshot(&snap, "flag_0").h, 24);
        assert_eq!(rect_px_from_snapshot(&snap, "side_face_0").h, 24);
        assert_eq!(rect_px_from_snapshot(&snap, "color_face_0").h, 24);
        assert_eq!(rect_px_from_snapshot(&snap, "ai_face_0").h, 24);
    }

    #[test]
    fn skirmish_lobby_form_is_centered_in_content_area() {
        let snap = solve_skirmish_lobby();
        let chrome = RightPanelChrome::shell_defaults();
        let panel_x = chrome.panel_x() as i32;
        let name = rect_px_from_snapshot(&snap, "player_name");
        let color = rect_px_from_snapshot(&snap, "color_face_0");
        let check = rect_px_from_snapshot(&snap, "checkbox_4");
        let left = name.x;
        let right = color.x + color.w;
        let right2 = check.x + check.w;
        let right = right.max(right2);
        let mid = (left + right) / 2;
        let content_mid = panel_x / 2;
        assert!(
            (mid - content_mid).abs() <= 4,
            "form mid {mid} should near content mid {content_mid} (left={left} right={right})"
        );
        // 不再贴顶：首行应明显低于旧 DLU y≈18。
        assert!(name.y >= 40, "player_name.y={} should leave top margin", name.y);
    }

    #[test]
    fn choose_map_has_lower_strip_and_bottom_status() {
        let snap = solve_choose_map();
        assert_eq!(
            rect_px_from_snapshot(&snap, "lower_strip"),
            RectPx::new(0, 568, 632, 32)
        );
        assert_eq!(
            rect_px_from_snapshot(&snap, "status_help"),
            RectPx::new(15, 579, 455, 20)
        );
    }

    #[test]
    fn choose_map_lists_are_centered_in_content_area() {
        let snap = solve_choose_map();
        let chrome = RightPanelChrome::shell_defaults();
        let panel_x = chrome.panel_x() as i32;
        let game_type = rect_px_from_snapshot(&snap, "game_type_list");
        let map_list = rect_px_from_snapshot(&snap, "map_list");
        let left = game_type.x;
        let right = map_list.x + map_list.w;
        let mid = (left + right) / 2;
        let content_mid = panel_x / 2;
        assert!(
            (mid - content_mid).abs() <= 4,
            "lists mid {mid} should near content mid {content_mid}"
        );
        assert!(game_type.y >= 40, "game_type_list.y={} should leave top margin", game_type.y);
        assert_eq!(map_list.x - (game_type.x + game_type.w), 27);
    }
}
