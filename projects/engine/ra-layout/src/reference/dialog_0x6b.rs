//! `RT_DIALOG` `0x6B`（选图）控件表 → 已解析设计矩形。

use crate::{
    geometry::{Rect, Size2},
    policy::{bottom_cover_button, right_panel_anchor, tile_snap_button, RightPanelChrome},
    reference::{DluRect, MS_SANS_SERIF_8PT},
    spec::{fixed_rect_leaf, root_with_fixed_children, LayoutNode},
};

/// 壳层设计视口。
pub fn shell_design_size(chrome: RightPanelChrome) -> Size2 {
    Size2 {
        width: chrome.shell_w,
        height: chrome.shell_h,
    }
}

fn dlu(x: i32, y: i32, w: i32, h: i32) -> Rect {
    DluRect::new(x, y, w, h).to_design_px(MS_SANS_SERIF_8PT)
}

/// 解析 `0x6B` 关键控件到设计像素（不含全页 chrome 装饰）。
pub fn resolve_dialog_0x6b(chrome: RightPanelChrome) -> Vec<(String, Rect)> {
    vec![
        (
            "use_map".into(),
            tile_snap_button(dlu(318, 122, 108, 23), chrome),
        ),
        (
            "create_random".into(),
            tile_snap_button(dlu(318, 149, 108, 23), chrome),
        ),
        ("cancel".into(), bottom_cover_button(chrome)),
        ("title".into(), right_panel_anchor(dlu(318, 1, 108, 10), chrome)),
        (
            "map_preview".into(),
            right_panel_anchor(dlu(324, 23, 96, 69), chrome),
        ),
        ("label_engagement".into(), dlu(23, 20, 257, 12)),
        ("game_type_list".into(), dlu(20, 78, 130, 160)),
        ("map_list".into(), dlu(168, 78, 130, 160)),
    ]
}

/// 组装可供 `LayoutEngine` 求解的控件树。
pub fn dialog_0x6b_layout_tree(chrome: RightPanelChrome) -> LayoutNode {
    let children = resolve_dialog_0x6b(chrome)
        .into_iter()
        .map(|(id, rect)| fixed_rect_leaf(id, rect))
        .collect();
    root_with_fixed_children("dialog_0x6b", shell_design_size(chrome), children)
}
