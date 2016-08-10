//! 自 `bindings/ra-napi/src/host/battle_controller/movement.rs` 迁出的单元测试（集成测试 crate）。

// 自 bindings/ra-napi/src/host/battle_controller/movement.rs :: slide_offset_tests
use ra_napi::host::battle_controller::movement::slide_offset_along_path;

#[test]
fn slide_offset_moves_toward_next_cell() {
    // 等距邻格：进度一半时应有明显非零偏移（非整格瞬移）。
    let path = [(2u16, 1u16)];
    let (ox0, oy0) = slide_offset_along_path(1, 1, &path, 0, 0, 0.0, 64, |_, _| 0);
    assert_eq!((ox0, oy0), (0, 0));
    let (ox, oy) = slide_offset_along_path(1, 1, &path, 32, 0, 0.0, 64, |_, _| 0);
    assert!(ox != 0 || oy != 0, "mid-cell slide must leave cell origin");
    let (ox1, oy1) = slide_offset_along_path(1, 1, &path, 64, 0, 0.0, 64, |_, _| 0);
    let (ox_half, oy_half) = (ox, oy);
    assert!(ox1.abs() >= ox_half.abs() || oy1.abs() >= oy_half.abs());
}

#[test]
fn tick_fraction_extends_slide_between_logic_ticks() {
    let path = [(2u16, 1u16)];
    let (a, b) = slide_offset_along_path(1, 1, &path, 0, 32, 0.0, 64, |_, _| 0);
    let (c, d) = slide_offset_along_path(1, 1, &path, 0, 32, 0.5, 64, |_, _| 0);
    assert_eq!((a, b), (0, 0));
    assert!(c != 0 || d != 0, "render fraction must advance slide without waiting for next logic tick");
}
