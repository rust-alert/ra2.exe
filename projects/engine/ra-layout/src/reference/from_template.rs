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
    }
}

/// 壳层设计视口。
pub fn shell_design_size(chrome: RightPanelChrome) -> Size2 {
    Size2 {
        width: chrome.shell_w,
        height: chrome.shell_h,
    }
}

/// 对话框整页：面板 chrome（无底条 / 标题 / 提示）+ 模板控件，一次求解。
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
