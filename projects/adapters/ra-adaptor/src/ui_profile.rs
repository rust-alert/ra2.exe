//! 壳层 `RuntimeUiProfile` 工厂（骨架：先填对话框 DLU 表）。

use ra_types::{
    ControlPlacement, DialogControlDesc, DialogTemplate, RuntimeUiProfile, UiCapabilities,
};

fn ctrl(id: &str, x: i32, y: i32, w: i32, h: i32, placement: ControlPlacement) -> DialogControlDesc {
    DialogControlDesc::with_placement(id, x, y, w, h, placement)
}

/// 选图对话框 `0x6B` 控件 DLU（与 `ra-layout` 金标一致）。
pub fn dialog_template_0x6b() -> DialogTemplate {
    DialogTemplate {
        dialog_id: 0x6B,
        controls: vec![
            ctrl("use_map", 318, 122, 108, 23, ControlPlacement::TileSnap),
            ctrl("create_random", 318, 149, 108, 23, ControlPlacement::TileSnap),
            // 取消钮不用模板 y，运行时贴底盖；DLU 仅作对照占位。
            ctrl("cancel", 318, 269, 108, 23, ControlPlacement::BottomCoverButton),
            ctrl("title", 318, 1, 108, 10, ControlPlacement::RightPanelAnchor),
            ctrl("map_preview", 324, 23, 96, 69, ControlPlacement::RightPanelAnchor),
            ctrl("label_engagement", 23, 20, 257, 12, ControlPlacement::PreserveDlu),
            ctrl("game_type_list", 20, 78, 130, 160, ControlPlacement::PreserveDlu),
            ctrl("map_list", 168, 78, 130, 160, ControlPlacement::PreserveDlu),
        ],
    }
}

/// 遭遇战大厅对话框 `0x102` 关键控件 DLU。
pub fn dialog_template_0x102() -> DialogTemplate {
    DialogTemplate {
        dialog_id: 0x102,
        controls: vec![
            ctrl("start", 318, 149, 108, 23, ControlPlacement::TileSnap),
            ctrl("choose_map", 318, 176, 108, 23, ControlPlacement::TileSnap),
            ctrl("back", 318, 269, 108, 23, ControlPlacement::BottomCoverButton),
            ctrl("title", 318, 1, 108, 10, ControlPlacement::RightPanelAnchor),
            ctrl("map_preview", 324, 23, 96, 69, ControlPlacement::RightPanelAnchor),
            ctrl("map_name_plate", 0, 0, 0, 0, ControlPlacement::MapNamePlate),
            ctrl("player_name", 35, 11, 100, 12, ControlPlacement::PreserveDlu),
            ctrl("checkbox_quick", 35, 145, 100, 10, ControlPlacement::PreserveDlu),
            ctrl("track_speed", 214, 145, 85, 13, ControlPlacement::PreserveDlu),
            ctrl("label_speed", 146, 145, 60, 10, ControlPlacement::PreserveDlu),
        ],
    }
}

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
            choose.controls.iter().find(|c| c.id.0 == "cancel").map(|c| c.placement),
            Some(ControlPlacement::BottomCoverButton)
        );
    }
}
