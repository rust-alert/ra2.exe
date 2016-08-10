//! 自 `engine/ra-types/src/definition/foundation.rs` 迁出的单元测试（集成测试 crate）。

// 自 engine/ra-types/src/definition/foundation.rs :: tests
use ra_types::definition::foundation::Foundation;

#[test]
fn parses_plain_wh() {
    let f = Foundation::parse("2x3");
    assert_eq!((f.width, f.height), (2, 3));
    assert_eq!(f.raw, "2X3");
}

#[test]
fn parses_wh_with_suffix() {
    let f = Foundation::parse("3x5Refinery");
    assert_eq!((f.width, f.height), (3, 5));
}

#[test]
fn unknown_falls_back_to_1x1() {
    let f = Foundation::parse("GateNE");
    assert_eq!((f.width, f.height), (1, 1));
    assert_eq!(f.raw, "GATENE");
}

#[test]
fn empty_is_default() {
    assert_eq!(Foundation::parse(""), Foundation::default());
}
