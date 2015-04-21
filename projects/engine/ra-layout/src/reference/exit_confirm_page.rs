//! 退出确认对话框 → `LayoutNode`。

use crate::{
    geometry::Rect,
    policy::RightPanelChrome,
    reference::{from_template::shell_design_size, DluRect, MS_SANS_SERIF_8PT},
    spec::{fixed_rect_leaf, root_with_fixed_children, LayoutNode},
    ui_layout::{
        EXIT_CONFIRM_BUTTON_H, EXIT_CONFIRM_BUTTON_IDS, EXIT_CONFIRM_BUTTON_W, EXIT_CONFIRM_DIALOG_H,
        EXIT_CONFIRM_DIALOG_W,
    },
};

fn centered_dialog(chrome: RightPanelChrome) -> Rect {
    let x = (((chrome.shell_w as i32 - EXIT_CONFIRM_DIALOG_W) + 1) / 2).max(0) as f32;
    let y = (((chrome.shell_h as i32 - EXIT_CONFIRM_DIALOG_H) + 1) / 2).max(0) as f32;
    Rect::from_xywh(
        x,
        y,
        EXIT_CONFIRM_DIALOG_W as f32,
        EXIT_CONFIRM_DIALOG_H as f32,
    )
}

fn modal_child(dialog: Rect, local: Rect) -> Rect {
    Rect::from_xywh(
        dialog.x + local.x,
        dialog.y + local.y,
        local.width,
        local.height,
    )
}

fn dlu_local(x: i32, y: i32, w: i32, h: i32) -> Rect {
    DluRect::new(x, y, w, h).to_design_px(MS_SANS_SERIF_8PT)
}

/// 退出确认：底板、提示区、确定/取消。
pub fn exit_confirm_content_layout_tree(chrome: RightPanelChrome) -> LayoutNode {
    let dialog = centered_dialog(chrome);
    let prompt = modal_child(dialog, dlu_local(40, 40, 220, 50));
    let ok_origin = modal_child(dialog, dlu_local(207, 135, 83, 15));
    let cancel_origin = modal_child(dialog, dlu_local(207, 175, 83, 15));
    let ok = Rect::from_xywh(
        ok_origin.x,
        ok_origin.y,
        EXIT_CONFIRM_BUTTON_W as f32,
        EXIT_CONFIRM_BUTTON_H as f32,
    );
    let cancel = Rect::from_xywh(
        cancel_origin.x,
        cancel_origin.y,
        EXIT_CONFIRM_BUTTON_W as f32,
        EXIT_CONFIRM_BUTTON_H as f32,
    );
    let children = vec![
        fixed_rect_leaf("dialog", dialog),
        fixed_rect_leaf("prompt", prompt),
        fixed_rect_leaf(EXIT_CONFIRM_BUTTON_IDS[0], ok),
        fixed_rect_leaf(EXIT_CONFIRM_BUTTON_IDS[1], cancel),
    ];
    root_with_fixed_children("exit_confirm", shell_design_size(chrome), children)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{LayoutEngine, Viewport};

    #[test]
    fn exit_confirm_content_matches_ui_layout_golden() {
        let chrome = RightPanelChrome::shell_defaults();
        let snap = LayoutEngine.solve(
            Viewport {
                size: shell_design_size(chrome),
                ..Viewport::default()
            },
            &exit_confirm_content_layout_tree(chrome),
        );
        let legacy = crate::ui_layout::exit_confirm_layout(800, 600);
        assert_eq!(
            snap.get("dialog").map(|e| (
                e.layout.rect.x as i32,
                e.layout.rect.y as i32,
                e.layout.rect.width as i32,
                e.layout.rect.height as i32
            )),
            Some((legacy.dialog.x, legacy.dialog.y, legacy.dialog.w, legacy.dialog.h))
        );
        assert_eq!(
            snap.get("ok").map(|e| (
                e.layout.rect.x as i32,
                e.layout.rect.y as i32,
                e.layout.rect.width as i32,
                e.layout.rect.height as i32
            )),
            Some((
                legacy.buttons[0].x,
                legacy.buttons[0].y,
                legacy.buttons[0].w,
                legacy.buttons[0].h
            ))
        );
        assert_eq!(
            snap.get("cancel").map(|e| (
                e.layout.rect.x as i32,
                e.layout.rect.y as i32,
                e.layout.rect.width as i32,
                e.layout.rect.height as i32
            )),
            Some((
                legacy.buttons[1].x,
                legacy.buttons[1].y,
                legacy.buttons[1].w,
                legacy.buttons[1].h
            ))
        );
    }
}
