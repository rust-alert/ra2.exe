//! 桌面配置：委托 `ra-config`。

pub use ra_config::{ConfigDiagnostic, DesktopSettings};

/// 桌面启动配置（与历史 `DesktopConfig` 同义）。
pub type DesktopConfig = DesktopSettings;

pub fn load_desktop_config_with_diagnostics() -> (DesktopConfig, Vec<ConfigDiagnostic>) {
    DesktopSettings::load_or_default()
}
