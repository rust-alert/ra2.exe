//! 退出确认对话框 → `LayoutNode`。

use crate::{
    geometry::Rect,
    policy::RightPanelChrome,
    reference::{
        DluRect, MS_SANS_SERIF_8PT,
        from_template::shell_design_size,
        shell_chrome::{shell_page_layout_tree, solve_with_shell_defaults},
    },
    shell::{
        EXIT_CONFIRM_BUTTON_H, EXIT_CONFIRM_BUTTON_IDS, EXIT_CONFIRM_BUTTON_W, EXIT_CONFIRM_DIALOG_H, EXIT_CONFIRM_DIALOG_W,
        MAIN_MENU_BUTTON_IDS,
    },
    snapshot::LayoutSnapshot,
    spec::{LayoutNode, fixed_rect_leaf, root_with_fixed_children},
};

pub fn centered_dialog(chrome: RightPanelChrome) -> Rect {
    let x = (((chrome.shell_w as i32 - EXIT_CONFIRM_DIALOG_W) + 1) / 2).max(0) as f32;
    let y = (((chrome.shell_h as i32 - EXIT_CONFIRM_DIALOG_H) + 1) / 2).max(0) as f32;
    Rect::from_xywh(x, y, EXIT_CONFIRM_DIALOG_W as f32, EXIT_CONFIRM_DIALOG_H as f32)
}

pub fn modal_child(dialog: Rect, local: Rect) -> Rect {
    Rect::from_xywh(dialog.x + local.x, dialog.y + local.y, local.width, local.height)
}

pub fn dlu_local(x: i32, y: i32, w: i32, h: i32) -> Rect {
    DluRect::new(x, y, w, h).to_design_px(MS_SANS_SERIF_8PT)
}

pub fn exit_confirm_modal_children(chrome: RightPanelChrome) -> Vec<LayoutNode> {
    let dialog = centered_dialog(chrome);
    let prompt = modal_child(dialog, dlu_local(40, 40, 220, 50));
    let ok_origin = modal_child(dialog, dlu_local(207, 135, 83, 15));
    let cancel_origin = modal_child(dialog, dlu_local(207, 175, 83, 15));
    let ok = Rect::from_xywh(ok_origin.x, ok_origin.y, EXIT_CONFIRM_BUTTON_W as f32, EXIT_CONFIRM_BUTTON_H as f32);
    let cancel = Rect::from_xywh(cancel_origin.x, cancel_origin.y, EXIT_CONFIRM_BUTTON_W as f32, EXIT_CONFIRM_BUTTON_H as f32);
    vec![
        fixed_rect_leaf("dialog", dialog),
        fixed_rect_leaf("prompt", prompt),
        fixed_rect_leaf(EXIT_CONFIRM_BUTTON_IDS[0], ok),
        fixed_rect_leaf(EXIT_CONFIRM_BUTTON_IDS[1], cancel),
    ]
}

/// 退出确认整页：主菜单壳（右栏六钮）+ 居中 MessageBox，一次求解。
pub fn exit_confirm_content_layout_tree(chrome: RightPanelChrome) -> LayoutNode {
    let mut children = shell_page_layout_tree("exit_confirm", &MAIN_MENU_BUTTON_IDS[..5], Some(MAIN_MENU_BUTTON_IDS[5]), chrome).children;
    children.extend(exit_confirm_modal_children(chrome));
    root_with_fixed_children("exit_confirm", shell_design_size(chrome), children)
}

/// 用壳层默认 chrome 求解 [`exit_confirm_content_layout_tree`]。
pub fn solve_exit_confirm() -> LayoutSnapshot {
    solve_with_shell_defaults(exit_confirm_content_layout_tree)
}
