//! 壳层 `RuntimeUiProfile` 工厂（骨架：先填对话框 DLU 表）。

use ra_types::{DialogControlDesc, DialogTemplate, RuntimeUiProfile, UiCapabilities};

fn ctrl(id: &str, x: i32, y: i32, w: i32, h: i32) -> DialogControlDesc {
    DialogControlDesc::preserve(id, x, y, w, h)
}

/// 选图对话框 `0x6B` 控件 DLU（与 `ra-layout` 金标一致）。
pub fn dialog_template_0x6b() -> DialogTemplate {
    DialogTemplate {
        dialog_id: 0x6B,
        controls: vec![
            ctrl("use_map", 318, 122, 108, 23),
            ctrl("create_random", 318, 149, 108, 23),
            // 取消钮不用模板 y，运行时贴底盖；此处保留占位便于对照。
            ctrl("cancel", 318, 269, 108, 23),
            ctrl("title", 318, 1, 108, 10),
            ctrl("map_preview", 324, 23, 96, 69),
            ctrl("label_engagement", 23, 20, 257, 12),
            ctrl("game_type_list", 20, 78, 130, 160),
            ctrl("map_list", 168, 78, 130, 160),
        ],
    }
}

/// 遭遇战大厅对话框 `0x102` 关键控件 DLU。
pub fn dialog_template_0x102() -> DialogTemplate {
    DialogTemplate {
        dialog_id: 0x102,
        controls: vec![
            ctrl("start", 318, 149, 108, 23),
            ctrl("choose_map", 318, 176, 108, 23),
            ctrl("back", 318, 269, 108, 23),
            ctrl("title", 318, 1, 108, 10),
            ctrl("map_preview", 324, 23, 96, 69),
            ctrl("player_name", 35, 11, 100, 12),
            ctrl("checkbox_quick", 35, 145, 100, 10),
            ctrl("track_speed", 214, 145, 85, 13),
            ctrl("label_speed", 146, 145, 60, 10),
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
    }
}
