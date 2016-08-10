//! 自 `adapters/ra-adaptor-phobos/src/lib.rs` 迁出的单元测试（集成测试 crate）。

// 自 adapters/ra-adaptor-phobos/src/lib.rs :: tests
use ra_adaptor_phobos::*;

#[test]
fn mo_layout_uses_artmo_ini() {
    let p = mo_layout_profile();
    assert_eq!(p.rules_ini, "rulesmo.ini");
    assert_eq!(p.art_ini, "artmo.ini");
}
