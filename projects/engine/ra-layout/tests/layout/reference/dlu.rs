//! 自 `engine/ra-layout/src/reference/dlu.rs` 迁出的单元测试（集成测试 crate）。

// 自 engine/ra-layout/src/reference/dlu.rs :: tests
use ra_layout::reference::dlu::*;

#[test]
fn ms_sans_8pt_sample_matches_shell_helpers() {
    let r = DluRect::new(318, 122, 108, 23).to_design_px(MS_SANS_SERIF_8PT);
    assert_eq!(r.x, 477.0);
    assert_eq!(r.y, 198.0);
    assert_eq!(r.width, 162.0);
    assert_eq!(r.height, 37.0);
}

#[test]
fn mul_div_round_negative() {
    assert_eq!(mul_div_round(-5, 6, 4), -8);
    assert_eq!(mul_div_round(-1, 13, 8), -2);
}
