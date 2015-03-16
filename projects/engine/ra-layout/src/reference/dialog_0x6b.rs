//! `RT_DIALOG` `0x6B`（选图）模板与布局树。

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

/// 选图对话框模板（与 adaptor 壳层 profile 对齐）。
pub fn dialog_template_0x6b() -> DialogTemplate {
    DialogTemplate {
        dialog_id: 0x6B,
        controls: vec![
            ctrl("use_map", 318, 122, 108, 23, ControlPlacement::TileSnap),
            ctrl("create_random", 318, 149, 108, 23, ControlPlacement::TileSnap),
            ctrl("cancel", 318, 269, 108, 23, ControlPlacement::BottomCoverButton),
            ctrl("title", 318, 1, 108, 10, ControlPlacement::RightPanelAnchor),
            ctrl("map_preview", 324, 23, 96, 69, ControlPlacement::RightPanelAnchor),
            ctrl("label_engagement", 23, 20, 257, 12, ControlPlacement::PreserveDlu),
            ctrl("game_type_list", 20, 78, 130, 160, ControlPlacement::PreserveDlu),
            ctrl("map_list", 168, 78, 130, 160, ControlPlacement::PreserveDlu),
        ],
    }
}

/// 解析 `0x6B` 关键控件到设计像素。
pub fn resolve_dialog_0x6b(chrome: RightPanelChrome) -> Vec<(String, Rect)> {
    resolve_dialog_template(&dialog_template_0x6b(), chrome)
}

/// 组装可供 `LayoutEngine` 求解的控件树。
pub fn dialog_0x6b_layout_tree(chrome: RightPanelChrome) -> LayoutNode {
    dialog_layout_tree("dialog_0x6b", &dialog_template_0x6b(), chrome)
}
