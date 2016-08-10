//! 自 `engine/ra-types/src/ui_profile.rs` 迁出的单元测试（集成测试 crate）。

// 自 engine/ra-types/src/ui_profile.rs :: tests
use ra_types::ui_profile::*;

#[test]
fn empty_profile_has_no_dialogs() {
    let profile = RuntimeUiProfile::default();
    assert!(profile.dialog(0x6B).is_none());
}

#[test]
fn dialog_lookup_by_id() {
    let profile = RuntimeUiProfile {
        dialog_templates: vec![DialogTemplate {
            dialog_id: 0x6B,
            controls: vec![DialogControlDesc::with_placement("use_map", 318, 122, 108, 23, ControlPlacement::TileSnap)],
        }],
        ..RuntimeUiProfile::default()
    };
    assert_eq!(profile.dialog(0x6B).map(|t| t.controls.len()), Some(1));
    assert_eq!(profile.dialog(0x6B).unwrap().controls[0].placement, ControlPlacement::TileSnap);
}

#[test]
fn shell_dialog_templates_expose_expected_ids() {
    assert!(dialog_template_0x6b().controls.iter().any(|c| c.id.0 == "map_list"));
    assert!(dialog_template_0x102().controls.iter().any(|c| c.id.0 == "start"));
    assert!(dialog_template_0x102().controls.iter().any(|c| c.id.0 == "flag_0"));
    assert!(dialog_template_0x102().controls.iter().any(|c| c.id.0 == "ai_face_6"));
}
