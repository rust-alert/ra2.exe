//! 自 `engine/ra-map/src/scripting/capability.rs` 迁出的单元测试（集成测试 crate）。

// 自 engine/ra-map/src/scripting/capability.rs :: tests
use ra_map::MapActionKind;

#[test]
fn supported_action_kinds_cover_enum_table() {
    assert!(MapActionKind::Win.is_supported());
    assert!(MapActionKind::DestroyAllLandUnitsOf.is_supported());
    assert!(!MapActionKind::from_code(99).is_supported());
    assert!(matches!(MapActionKind::from_code(99), MapActionKind::Unknown(99)));
    assert_eq!(MapActionKind::SUPPORTED.len(), 43);
}
