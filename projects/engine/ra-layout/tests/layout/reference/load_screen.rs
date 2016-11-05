//! 自 `engine/ra-layout/src/reference/load_screen.rs` 迁出的单元测试（集成测试 crate）。

// 自 engine/ra-layout/src/reference/load_screen.rs :: tests
use ra_layout::{LayoutEngine, RightPanelChrome, Size2, Viewport, reference::load_screen::*};

#[test]
fn load_screen_tree_matches_compose_constants() {
    let chrome = RightPanelChrome::shell_defaults();
    let snap = LayoutEngine.solve(
        Viewport { size: Size2 { width: chrome.shell_w, height: chrome.shell_h }, ..Viewport::default() },
        &load_screen_layout_tree(chrome),
    );
    for (id, x, y, w, h) in [
        ("special", 54, 106, 190, 22),
        ("brief", 48, 134, 340, 200),
        ("name", 648, 538, 120, 24),
        ("status", 56, 310, 160, 20),
        ("progress", 56, 332, 200, 16),
        ("player_flag", 150, 324, 40, 24),
        ("player_name", 202, 328, 120, 20),
        ("map_preview", 499, 379, 216, 166),
    ] {
        let got = snap.get(id).expect(id).layout.rect;
        assert_eq!(got.x as i32, x, "{id} x");
        assert_eq!(got.y as i32, y, "{id} y");
        assert_eq!(got.width as i32, w, "{id} w");
        assert_eq!(got.height as i32, h, "{id} h");
    }
    let retry = snap.get("retry").unwrap().layout.rect;
    assert_eq!(retry.x as i32, 240);
    assert_eq!(retry.y as i32, 528);
    assert_eq!(retry.width as i32, 160);
    assert_eq!(retry.height as i32, 48);
    let cancel = snap.get("cancel").unwrap().layout.rect;
    assert_eq!(cancel.x as i32, 432);
    assert_eq!(cancel.y as i32, 528);
}
