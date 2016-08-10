//! 自 `engine/ra-layout/src/reference/exit_confirm_page.rs` 迁出的单元测试（集成测试 crate）。

// 自 engine/ra-layout/src/reference/exit_confirm_page.rs :: tests
use ra_layout::{
    EXIT_CONFIRM_BUTTON_H, EXIT_CONFIRM_BUTTON_W, EXIT_CONFIRM_DIALOG_H, EXIT_CONFIRM_DIALOG_W, LayoutEngine, MAIN_MENU_BUTTON_IDS,
    RightPanelChrome, Viewport,
    reference::{exit_confirm_page::*, from_template::shell_design_size},
};

#[test]
fn exit_confirm_content_matches_shell_golden() {
    let chrome = RightPanelChrome::shell_defaults();
    let snap =
        LayoutEngine.solve(Viewport { size: shell_design_size(chrome), ..Viewport::default() }, &exit_confirm_content_layout_tree(chrome));
    assert_eq!(
        snap.get("dialog").map(|e| (e.layout.rect.x as i32, e.layout.rect.y as i32, e.layout.rect.width as i32, e.layout.rect.height as i32)),
        Some((175, 137, EXIT_CONFIRM_DIALOG_W, EXIT_CONFIRM_DIALOG_H))
    );
    assert_eq!(
        snap.get("ok").map(|e| (e.layout.rect.x as i32, e.layout.rect.y as i32, e.layout.rect.width as i32, e.layout.rect.height as i32)),
        Some((486, 356, EXIT_CONFIRM_BUTTON_W, EXIT_CONFIRM_BUTTON_H))
    );
    assert_eq!(
        snap.get("cancel").map(|e| (e.layout.rect.x as i32, e.layout.rect.y as i32, e.layout.rect.width as i32, e.layout.rect.height as i32)),
        Some((486, 421, EXIT_CONFIRM_BUTTON_W, EXIT_CONFIRM_BUTTON_H))
    );
    let shell_snap = ra_layout::solve_shell_page("main_menu", &MAIN_MENU_BUTTON_IDS[..5], Some(MAIN_MENU_BUTTON_IDS[5]));
    assert_eq!(snap.get("title").map(|e| e.layout.rect), shell_snap.get("title").map(|e| e.layout.rect));
    assert_eq!(snap.get("single_player").map(|e| e.layout.rect), shell_snap.get("single_player").map(|e| e.layout.rect));
}
