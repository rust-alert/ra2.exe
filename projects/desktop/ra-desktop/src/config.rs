//! 桌面配置：委托 `ra-config`。

pub use ra_config::{ConfigDiagnostic, DesktopSettings, LaunchOverride};

/// 桌面启动配置（与历史 `DesktopConfig` 同义）。
pub type DesktopConfig = DesktopSettings;

pub fn load_desktop_config_with_diagnostics() -> (DesktopConfig, Vec<ConfigDiagnostic>) {
    DesktopSettings::load_or_default()
}

/// 设置一次性启动覆盖（`ra2 launch --path` / N-API）。
pub fn set_launch_override(ra2_dir: impl Into<std::path::PathBuf>, edition: Option<String>) {
    ra_config::set_launch_override(LaunchOverride { ra2_dir: ra2_dir.into(), edition });
}

/// 清除启动覆盖。
pub fn clear_launch_override() {
    ra_config::clear_launch_override();
}
