//! 自 `engine/ra-map/src/playfield.rs` 迁出的单元测试（集成测试 crate）。

// 自 engine/ra-map/src/playfield.rs :: tests
use ra_map::playfield::*;

#[test]
fn dustbowl_style_center_cell_inside_local_playfield() {
    // Size=70×76 LocalSize=2,8,65,62 → 中心附近应在可见区内。
    let local = LocalSize { left: 2, top: 8, width: 65, height: 62 };
    assert!(cell_in_local_playfield(70, local, 74, 75));
    assert!(!cell_in_local_playfield(70, local, 0, 0));
}

#[test]
fn local_size_preview_rect_is_finite() {
    let local = LocalSize { left: 5, top: 4, width: 95, height: 85 };
    let rect = local_size_preview_rect(105, local, -1000, -500).expect("rect");
    assert!(rect.2 > rect.0);
    assert!(rect.3 > rect.1);
}
