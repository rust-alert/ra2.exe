//! `RT_DIALOG` `0x102`（遭遇战大厅）控件表 → 已解析设计矩形。

use crate::{
    geometry::{Rect, Size2},
    policy::{bottom_cover_button, right_panel_anchor, tile_snap_button, RightPanelChrome},
    reference::{DluRect, MS_SANS_SERIF_8PT},
    spec::{fixed_rect_leaf, root_with_fixed_children, LayoutNode},
};

fn dlu(x: i32, y: i32, w: i32, h: i32) -> Rect {
    DluRect::new(x, y, w, h).to_design_px(MS_SANS_SERIF_8PT)
}

/// 地图名底板（贴右缘，底边落在第一根 tile 下沿）。
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

/// 解析 `0x102` 关键控件到设计像素。
pub fn resolve_dialog_0x102(chrome: RightPanelChrome) -> Vec<(String, Rect)> {
    vec![
        (
            "start".into(),
            tile_snap_button(dlu(318, 149, 108, 23), chrome),
        ),
        (
            "choose_map".into(),
            tile_snap_button(dlu(318, 176, 108, 23), chrome),
        ),
        ("back".into(), bottom_cover_button(chrome)),
        ("title".into(), right_panel_anchor(dlu(318, 1, 108, 10), chrome)),
        (
            "map_preview".into(),
            right_panel_anchor(dlu(324, 23, 96, 69), chrome),
        ),
        ("map_name_plate".into(), map_name_plate(chrome)),
        ("player_name".into(), dlu(35, 11, 100, 12)),
        ("checkbox_quick".into(), dlu(35, 145, 100, 10)),
        ("track_speed".into(), dlu(214, 145, 85, 13)),
        ("label_speed".into(), dlu(146, 145, 60, 10)),
    ]
}

/// 组装可供 `LayoutEngine` 求解的控件树。
pub fn dialog_0x102_layout_tree(chrome: RightPanelChrome) -> LayoutNode {
    let children = resolve_dialog_0x102(chrome)
        .into_iter()
        .map(|(id, rect)| fixed_rect_leaf(id, rect))
        .collect();
    root_with_fixed_children(
        "dialog_0x102",
        Size2 {
            width: chrome.shell_w,
            height: chrome.shell_h,
        },
        children,
    )
}
