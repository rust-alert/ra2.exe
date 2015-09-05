//! 退出确认对话框布局。

use super::*;
use crate::solve_exit_confirm;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ExitConfirmLayout {
    /// 对话框底板（`pudlgbgn`）。
    pub dialog: RectPx,
    /// 提示文案区（左上锚点，非居中）。
    pub prompt: RectPx,
    /// 确定 / 取消（DLU 控件格；`mnbttn` 自左上贴齐，可溢出 1px）。
    pub buttons: [RectPx; 2],
}

/// 退出确认布局（相对壳层画布居中；底下仍是主菜单右栏）。
///
/// 交互几何来自 `exit_confirm_content_layout_tree`。
pub fn exit_confirm_layout(_viewport_w: u32, _viewport_h: u32) -> ExitConfirmLayout {
    let snap = solve_exit_confirm();
    ExitConfirmLayout {
        dialog: rect_px_from_snapshot(&snap, "dialog"),
        prompt: rect_px_from_snapshot(&snap, "prompt"),
        buttons: [
            rect_px_from_snapshot(&snap, EXIT_CONFIRM_BUTTON_IDS[0]),
            rect_px_from_snapshot(&snap, EXIT_CONFIRM_BUTTON_IDS[1]),
        ],
    }
}
