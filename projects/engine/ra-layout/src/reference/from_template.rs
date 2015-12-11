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
    let mut children = shell_panel_chrome_children(chrome);
    children.extend(dialog_control_children(template, chrome));
    root_with_fixed_children(root_id, shell_design_size(chrome), children)
}

fn dialog_control_children(template: &DialogTemplate, chrome: RightPanelChrome) -> Vec<LayoutNode> {
    let mut controls: Vec<&DialogControlDesc> = template.controls.iter().collect();
    controls.sort_by_key(|c| match c.placement {
        ControlPlacement::TileSnap | ControlPlacement::BottomCoverButton => 1_u8,
        _ => 0,
    });
    controls
        .into_iter()
        .map(|c| fixed_rect_leaf(c.id.0.clone(), resolve_control_desc(c, chrome)))
        .collect()
}

/// 选图页：面板 chrome + `0x6B` 模板 → 一次 snapshot。
pub fn solve_choose_map() -> LayoutSnapshot {
    solve_with_shell_defaults(|chrome| {
        dialog_page_layout_tree("dialog_0x6b", &dialog_template_0x6b(), chrome)
    })
}

/// 遭遇战大厅：面板 chrome + `0x102` 模板 → 一次 snapshot。
pub fn solve_skirmish_lobby() -> LayoutSnapshot {
    solve_with_shell_defaults(|chrome| {
        dialog_page_layout_tree("dialog_0x102", &dialog_template_0x102(), chrome)
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
        // 同行控件同高，避免名框 / 旗标矮于下拉面。
        assert_eq!(rect_px_from_snapshot(&snap, "player_name").h, 24);
        assert_eq!(rect_px_from_snapshot(&snap, "flag_0").h, 24);
        assert_eq!(rect_px_from_snapshot(&snap, "side_face_0").h, 24);
        assert_eq!(rect_px_from_snapshot(&snap, "color_face_0").h, 24);
        assert_eq!(rect_px_from_snapshot(&snap, "ai_face_0").h, 24);
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
}
