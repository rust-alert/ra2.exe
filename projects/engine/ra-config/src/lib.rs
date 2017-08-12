//! 配置与用户状态：平台用户数据目录（或 Web `localStorage`）下的 JSON。
//!
//! - **settings.json**：启动 / 选项配置（`DesktopSettings`）
//! - **state.json**：可变用户状态（`DesktopState`，含遭遇战大厅记忆）
//!
//! 目录名统一为 `rust-alert2`。本 crate 不解释游戏语义。

#![deny(missing_docs)]

mod paths;
mod settings;
mod state;
mod store;

use std::{
    collections::BTreeMap,
    path::PathBuf,
    sync::Mutex,
};

pub use paths::{
    APP_DATA_DIR_NAME, SETTINGS_FILE_NAME, STATE_FILE_NAME, WEB_STORAGE_PREFIX, ensure_user_data_dir, exe_dir,
    set_test_user_data_dir, settings_path, state_path, user_data_dir, user_data_join,
};
pub use settings::DesktopSettings;
pub use state::{DesktopState, SkirmishLobbyPrefs};
pub use store::{LocalStorageStore, NativeDirStore, PersistStore, default_store, persist_location_label};

/// CLI / N-API 一次性启动覆盖（后于 settings.json 生效，不进 state）。
#[derive(Debug, Clone)]
pub struct EmulateOverride {
    /// 游戏安装目录。
    pub ra2_dir: PathBuf,
    /// 可选版本字符串。
    pub edition: Option<String>,
    /// 可选启动产品页别名（如 `skirmish`；仅 CLI / N-API，不进 settings）。
    pub screen: Option<String>,
}

pub(crate) static EMULATE_OVERRIDE: Mutex<Option<EmulateOverride>> = Mutex::new(None);

/// 设置启动覆盖（`ra2 emulate --path`）。
pub fn set_emulate_override(override_: EmulateOverride) {
    *EMULATE_OVERRIDE.lock().expect("emulate override lock") = Some(override_);
}

/// 清除启动覆盖。
pub fn clear_emulate_override() {
    *EMULATE_OVERRIDE.lock().expect("emulate override lock") = None;
}

/// 读取 CLI / N-API 启动页覆盖（不消费；不进 settings；无覆盖或空串时为 `None`）。
pub fn emulate_override_screen() -> Option<String> {
    EMULATE_OVERRIDE
        .lock()
        .expect("emulate override lock")
        .as_ref()
        .and_then(|o| o.screen.as_ref())
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
}

/// 一条配置诊断。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConfigDiagnostic {
    /// 来源标签（如文件路径或 `defaults`）。
    pub source: String,
    /// 人类可读说明。
    pub message: String,
}

/// 扁平字符串配置表（测试合并用）。
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ConfigTable {
    values: BTreeMap<String, String>,
}

impl ConfigTable {
    /// 空表。
    pub fn new() -> Self {
        Self::default()
    }

    /// 按键取值。
    pub fn get(&self, key: &str) -> Option<&str> {
        self.values.get(key).map(|s| s.as_str())
    }

    /// 插入或覆盖。
    pub fn insert(&mut self, key: impl Into<String>, value: impl Into<String>) {
        self.values.insert(key.into(), value.into());
    }

    /// 遍历键值（按键排序）。
    pub fn iter(&self) -> impl Iterator<Item = (&str, &str)> {
        self.values.iter().map(|(k, v)| (k.as_str(), v.as_str()))
    }
}

/// 带来源标签的一层配置。
#[derive(Debug, Clone)]
pub struct ConfigLayer {
    /// 层标签。
    pub label: String,
    /// 该层键值。
    pub table: ConfigTable,
}

/// 合并结果：后者覆盖前者同名键。
#[derive(Debug, Clone, Default)]
pub struct MergedConfig {
    /// 合并后的表。
    pub table: ConfigTable,
    /// 合并过程中的诊断。
    pub diagnostics: Vec<ConfigDiagnostic>,
}

impl MergedConfig {
    /// 按顺序合并多层；后层覆盖前层。
    pub fn merge_layers(layers: &[ConfigLayer]) -> Self {
        let mut out = Self::default();
        for layer in layers {
            for (k, v) in layer.table.iter() {
                out.table.insert(k, v);
            }
        }
        out
    }

    /// 读取合并后的键。
    pub fn get(&self, key: &str) -> Option<&str> {
        self.table.get(key)
    }
}

/// 解析可选本机安装根目录（供 `#[ignore]` 本机测试 / 探针）。
///
/// 优先级：`RA2_DIR` → `settings.json` 的 `ra2_dir`。
/// 路径不存在则返回 `None`。**禁止**在调用方硬编码盘符或机主路径。
pub fn resolve_optional_install_root() -> Option<(PathBuf, Option<String>)> {
    if let Ok(dir) = std::env::var("RA2_DIR") {
        let root = PathBuf::from(dir.trim());
        if root.is_dir() {
            return Some((root, std::env::var("RA2_EDITION").ok().filter(|s| !s.trim().is_empty())));
        }
    }

    if let Ok(Some(text)) = store::NativeDirStore.read_text(SETTINGS_FILE_NAME) {
        let (settings, _) = DesktopSettings::from_json_text(&text, "settings.json");
        if settings.ra2_dir.is_dir() {
            return Some((settings.ra2_dir, settings.edition));
        }
    }

    None
}

/// 解析 0..1 音量；非法或非有限值返回 `None`。
pub fn parse_unit_volume(raw: &str) -> Option<f32> {
    let v: f32 = raw.trim().parse().ok()?;
    if v.is_finite() { Some(v.clamp(0.0, 1.0)) } else { None }
}
