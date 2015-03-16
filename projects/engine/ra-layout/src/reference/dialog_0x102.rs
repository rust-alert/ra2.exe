//! `RT_DIALOG` `0x102`（遭遇战大厅）模板与布局树。

use ra_types::{ControlPlacement, DialogControlDesc, DialogTemplate};

use crate::{
    geometry::Rect,
    policy::RightPanelChrome,
    reference::from_template::{dialog_layout_tree, resolve_dialog_template},
    spec::LayoutNode,
};

fn ctrl(id: &str, x: i32, y: i32, w: i32, h: i32, placement: ControlPlacement) -> DialogControlDesc {
    DialogControlDesc::with_placement(id, x, y, w, h, placement)
}

/// 遭遇战大厅对话框模板（与 adaptor 壳层 profile 对齐）。
pub fn dialog_template_0x102() -> DialogTemplate {
    DialogTemplate {
        dialog_id: 0x102,
        controls: vec![
            ctrl("start", 318, 149, 108, 23, ControlPlacement::TileSnap),
            ctrl("choose_map", 318, 176, 108, 23, ControlPlacement::TileSnap),
            ctrl("back", 318, 269, 108, 23, ControlPlacement::BottomCoverButton),
            ctrl("title", 318, 1, 108, 10, ControlPlacement::RightPanelAnchor),
            ctrl("map_preview", 324, 23, 96, 69, ControlPlacement::RightPanelAnchor),
            ctrl("map_name_plate", 0, 0, 0, 0, ControlPlacement::MapNamePlate),
            ctrl("player_name", 35, 11, 100, 12, ControlPlacement::PreserveDlu),
            ctrl("checkbox_quick", 35, 145, 100, 10, ControlPlacement::PreserveDlu),
            ctrl("track_speed", 214, 145, 85, 13, ControlPlacement::PreserveDlu),
            ctrl("label_speed", 146, 145, 60, 10, ControlPlacement::PreserveDlu),
        ],
    }
}

/// 解析 `0x102` 关键控件到设计像素。
pub fn resolve_dialog_0x102(chrome: RightPanelChrome) -> Vec<(String, Rect)> {
    resolve_dialog_template(&dialog_template_0x102(), chrome)
}

/// 组装可供 `LayoutEngine` 求解的控件树。
pub fn dialog_0x102_layout_tree(chrome: RightPanelChrome) -> LayoutNode {
    dialog_layout_tree("dialog_0x102", &dialog_template_0x102(), chrome)
}
