//! 壳层 `RuntimeUiProfile` 工厂。

use ra_types::{RuntimeUiProfile, UiCapabilities};

pub use ra_types::{dialog_template_0x6b, dialog_template_0x102};

/// 构造当前壳层可用的 UI profile（尚未接资源链探测）。
pub fn shell_runtime_ui_profile() -> RuntimeUiProfile {
    RuntimeUiProfile {
        dialog_templates: vec![dialog_template_0x6b(), dialog_template_0x102()],
        ui_capabilities: UiCapabilities { create_random_map: false, skirmish_lobby: true },
        ..RuntimeUiProfile::default()
    }
}
