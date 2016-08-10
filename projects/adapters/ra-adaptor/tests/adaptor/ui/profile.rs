//! 自 `adapters/ra-adaptor/src/ui_profile.rs` 迁出的单元测试（集成测试 crate）。

// 自 adapters/ra-adaptor/src/ui_profile.rs :: tests
use ra_adaptor::ui_profile::*;
use ra_types::ControlPlacement;

#[test]
fn shell_profile_exposes_choose_map_and_skirmish_dialogs() {
    let profile = shell_runtime_ui_profile();
    assert!(profile.dialog(0x6B).is_some());
    assert!(profile.dialog(0x102).is_some());
    assert!(profile.ui_capabilities.skirmish_lobby);
    assert!(!profile.ui_capabilities.create_random_map);
    let choose = profile.dialog(0x6B).unwrap();
    assert_eq!(choose.controls[0].placement, ControlPlacement::TileSnap);
    assert_eq!(choose.controls.iter().find(|c| c.id.0 == "cancel").map(|c| c.placement), Some(ControlPlacement::BottomCoverButton));
}
