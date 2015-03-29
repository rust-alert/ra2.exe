//! 壳层 `RuntimeUiProfile` 工厂。

use ra_types::{ControlPlacement, RuntimeUiProfile, UiCapabilities};

pub use ra_types::{dialog_template_0x102, dialog_template_0x6b};

/// 构造当前壳层可用的 UI profile（尚未接资源链探测）。
pub fn shell_runtime_ui_profile() -> RuntimeUiProfile {
    RuntimeUiProfile {
        dialog_templates: vec![dialog_template_0x6b(), dialog_template_0x102()],
        ui_capabilities: UiCapabilities {
            create_random_map: false,
            skirmish_lobby: true,
        },
        ..RuntimeUiProfile::default()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn shell_profile_exposes_choose_map_and_skirmish_dialogs() {
        let profile = shell_runtime_ui_profile();
        assert!(profile.dialog(0x6B).is_some());
        assert!(profile.dialog(0x102).is_some());
        assert!(profile.ui_capabilities.skirmish_lobby);
        assert!(!profile.ui_capabilities.create_random_map);
        let choose = profile.dialog(0x6B).unwrap();
        assert_eq!(choose.controls[0].placement, ControlPlacement::TileSnap);
        assert_eq!(
            choose
                .controls
                .iter()
                .find(|c| c.id.0 == "cancel")
                .map(|c| c.placement),
            Some(ControlPlacement::BottomCoverButton)
        );
    }
}
