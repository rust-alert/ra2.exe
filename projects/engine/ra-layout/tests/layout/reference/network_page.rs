//! 自 `engine/ra-layout/src/reference/network_page.rs` 迁出的单元测试（集成测试 crate）。

// 自 engine/ra-layout/src/reference/network_page.rs :: tests
use ra_layout::reference::network_page::*;

#[test]
fn network_page_matches_design_slot_rects() {
    let snap = solve_network_page();
    let online = snap.get("online").unwrap().layout.rect;
    assert_eq!(online.x as i32, 176);
    assert_eq!(online.y as i32, 216);
    assert_eq!(online.width as i32, 448);
    assert_eq!(online.height as i32, 48);
    let back = snap.get("back").unwrap().layout.rect;
    assert_eq!(back.x as i32, 176);
    assert_eq!(back.y as i32, 336);
    assert_eq!(back.width as i32, 448);
    assert_eq!(back.height as i32, 48);
}
