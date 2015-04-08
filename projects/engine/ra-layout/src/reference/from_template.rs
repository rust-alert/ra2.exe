//! 由 `DialogTemplate` + chrome 策略解析设计像素。

use ra_types::{ControlPlacement, DialogControlDesc, DialogTemplate};

use crate::{
    geometry::{Rect, Size2},
    policy::{bottom_cover_button, right_panel_anchor, tile_snap_button, RightPanelChrome},
    reference::{DluRect, MS_SANS_SERIF_8PT},
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
pub fn resolve_control_desc(desc: &DialogControlDesc, chrome: RightPanelChrome) -> Rect {
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

/// 解析整份对话框模板。
pub fn resolve_dialog_template(
    template: &DialogTemplate,
    chrome: RightPanelChrome,
) -> Vec<(String, Rect)> {
    template
        .controls
        .iter()
        .map(|c| (c.id.0.clone(), resolve_control_desc(c, chrome)))
        .collect()
}

/// 壳层设计视口。
pub fn shell_design_size(chrome: RightPanelChrome) -> Size2 {
    Size2 {
        width: chrome.shell_w,
        height: chrome.shell_h,
    }
}

/// 由模板组装 `LayoutNode` 树。
pub fn dialog_layout_tree(
    root_id: impl Into<String>,
    template: &DialogTemplate,
    chrome: RightPanelChrome,
) -> LayoutNode {
    let children = resolve_dialog_template(template, chrome)
        .into_iter()
        .map(|(id, rect)| fixed_rect_leaf(id, rect))
        .collect();
    root_with_fixed_children(root_id, shell_design_size(chrome), children)
}
