//! 退出确认对话框布局。

use super::*;
use crate::{solve_exit_confirm, RightPanelChrome};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ExitConfirmLayout {
    /// 对话框底板（`pudlgbgn`）。
    pub dialog: RectPx,
    /// 提示文案区（左上锚点，非居中）。
    pub prompt: RectPx,
    /// 确定 / 取消（DLU 控件格；`mnbttn` 自左上贴齐，可溢出 1px）。
    pub buttons: [RectPx; 2],
}

fn exit_confirm_from_snap(snap: &crate::LayoutSnapshot) -> ExitConfirmLayout {
    ExitConfirmLayout {
        dialog: rect_px_from_snapshot(snap, "dialog"),
        prompt: rect_px_from_snapshot(snap, "prompt"),
        buttons: [
            rect_px_from_snapshot(snap, EXIT_CONFIRM_BUTTON_IDS[0]),
            rect_px_from_snapshot(snap, EXIT_CONFIRM_BUTTON_IDS[1]),
        ],
    }
}

/// 退出确认整页：主菜单壳与居中对话框投影自同一次求解。
pub fn exit_confirm_page_layouts(
    _viewport_w: u32,
    _viewport_h: u32,
) -> (MainMenuLayout, ExitConfirmLayout) {
    let chrome = RightPanelChrome::shell_defaults();
    let snap = solve_exit_confirm();
    let mut shell = layout_from_shell_page_snap(chrome, &snap);
    let rail = buttons_from_snap(&snap, &MAIN_MENU_BUTTON_IDS);
    shell.buttons = [rail[0], rail[1], rail[2], rail[3], rail[4], rail[5]];
    (shell, exit_confirm_from_snap(&snap))
}

/// 退出确认布局（相对壳层画布居中；底下仍是主菜单右栏）。
///
/// 交互几何来自 `exit_confirm_content_layout_tree`（含主菜单壳）。
pub fn exit_confirm_layout(_viewport_w: u32, _viewport_h: u32) -> ExitConfirmLayout {
    exit_confirm_from_snap(&solve_exit_confirm())
}
