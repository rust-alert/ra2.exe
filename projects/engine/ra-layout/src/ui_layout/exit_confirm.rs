//! 退出确认对话框布局。

use super::*;


#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ExitConfirmLayout {
    /// 对话框底板（`pudlgbgn`）。
    pub dialog: RectPx,
    /// 提示文案区（左上锚点，非居中）。
    pub prompt: RectPx,
    /// 确定 / 取消（DLU 控件格；`mnbttn` 自左上贴齐，可溢出 1px）。
    pub buttons: [RectPx; 2],
}

pub(super) fn modal_child(dialog: RectPx, local: RectPx) -> RectPx {
    RectPx::new(dialog.x + local.x, dialog.y + local.y, local.w, local.h)
}

/// 退出确认布局（相对壳层画布居中；底下仍是主菜单右栏）。
pub fn exit_confirm_layout(_viewport_w: u32, _viewport_h: u32) -> ExitConfirmLayout {
    let dialog = RectPx::new(
        (((SHELL_BASE_W - EXIT_CONFIRM_DIALOG_W) + 1) / 2).max(0),
        (((SHELL_BASE_H - EXIT_CONFIRM_DIALOG_H) + 1) / 2).max(0),
        EXIT_CONFIRM_DIALOG_W,
        EXIT_CONFIRM_DIALOG_H,
    );
    ExitConfirmLayout {
        dialog,
        prompt: modal_child(dialog, dlu_rect(40, 40, 220, 50)),
        // 控件原点取 DLU，宽高取 `mnbttn` 画布（126×25），避免 125×24 格内居中错位。
        buttons: [
            {
                let r = modal_child(dialog, dlu_rect(207, 135, 83, 15));
                RectPx::new(r.x, r.y, EXIT_CONFIRM_BUTTON_W, EXIT_CONFIRM_BUTTON_H)
            },
            {
                let r = modal_child(dialog, dlu_rect(207, 175, 83, 15));
                RectPx::new(r.x, r.y, EXIT_CONFIRM_BUTTON_W, EXIT_CONFIRM_BUTTON_H)
            },
        ],
    }
}
