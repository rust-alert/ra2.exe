//! 自 `engine/ra-map/src/scripting/capability.rs` 迁出的单元测试（集成测试 crate）。

use ra_map::MapActionKind;

#[test]
fn supported_action_kinds_cover_enum_table() {
    assert!(MapActionKind::Win.is_supported());
    assert!(MapActionKind::DestroyAllLandUnitsOf.is_supported());
    assert!(MapActionKind::from_code(99).is_supported());
    assert!(MapActionKind::from_code(99).is_presentation_stub());
    assert!(!MapActionKind::from_code(42).is_supported());
    assert!(matches!(MapActionKind::from_code(42), MapActionKind::Unknown(42)));
    assert_eq!(MapActionKind::SUPPORTED.len(), 57);
    assert!(!MapActionKind::CreateTeam.is_presentation_stub());
    assert!(MapActionKind::from_code(3).is_supported());
    assert!(!MapActionKind::from_code(3).is_presentation_stub());
    assert!(MapActionKind::LockInput.is_supported());
    assert!(!MapActionKind::LockInput.is_presentation_stub());
    assert!(MapActionKind::Apply100Damage.is_supported());
    assert!(!MapActionKind::Apply100Damage.is_presentation_stub());
}

#[test]
fn all01t_presentation_actions_are_recognized_stubs() {
    for code in [41, 48, 55, 99, 100, 104, 113, 114, 115, 116] {
        let kind = MapActionKind::from_code(code);
        assert!(kind.is_supported(), "code {code}");
        assert!(kind.is_presentation_stub(), "code {code}");
        assert_eq!(kind.code(), code);
    }
    for code in [46, 47, 63] {
        let kind = MapActionKind::from_code(code);
        assert!(kind.is_supported(), "code {code}");
        assert!(!kind.is_presentation_stub(), "code {code} is gameplay not stub");
    }
    for code in [108] {
        assert!(!MapActionKind::from_code(code).is_supported(), "gameplay code {code} must stay Unknown until implemented");
    }
}
