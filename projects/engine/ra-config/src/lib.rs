//! 配置与用户状态：平台用户数据目录（或 Web `localStorage`）下的 JSON。
//!
//! - **settings.json**：启动 / 选项配置（`DesktopSettings`）
//! - **state.json**：可变用户状态（`DesktopState`，含遭遇战大厅记忆）
//!
//! 目录名统一为 `rust-alert2`。本 crate 不解释游戏语义。

#![deny(missing_docs)]

mod legacy;
mod paths;
mod settings;
mod state;
mod store;

use std::{
    collections::BTreeMap,
    path::{Path, PathBuf},
    sync::Mutex,
};

use toml_edit::{DocumentMut, Item, Value};

pub use legacy::{LegacyMigration, migrate_toml_text_to_store, parse_ra2_dir_from_toml_text};
pub use paths::{
    APP_DATA_DIR_NAME, LEGACY_RUST_ALERT_TOML, SETTINGS_FILE_NAME, STATE_FILE_NAME, WEB_STORAGE_PREFIX, ensure_user_data_dir, exe_dir,
    legacy_rust_alert_toml_path, set_test_user_data_dir, settings_path, state_path, user_data_dir, user_data_join,
};
pub use settings::DesktopSettings;
pub use state::{DesktopState, SkirmishLobbyPrefs};
pub use store::{LocalStorageStore, NativeDirStore, PersistStore, default_store, persist_location_label};

/// CLI / N-API 一次性启动覆盖（后于 settings.json 生效，不进 state）。
#[derive(Debug, Clone)]
pub struct LaunchOverride {
    /// 游戏安装目录。
    pub ra2_dir: PathBuf,
    /// 可选版本字符串。
    pub edition: Option<String>,
    /// 可选启动产品页别名（如 `skirmish`；仅 CLI / N-API，不进 settings）。
    pub screen: Option<String>,
}

pub(crate) static LAUNCH_OVERRIDE: Mutex<Option<LaunchOverride>> = Mutex::new(None);

/// 设置启动覆盖（`ra2 launch --path`）。
pub fn set_launch_override(override_: LaunchOverride) {
    *LAUNCH_OVERRIDE.lock().expect("launch override lock") = Some(override_);
}

/// 清除启动覆盖。
pub fn clear_launch_override() {
    *LAUNCH_OVERRIDE.lock().expect("launch override lock") = None;
}

/// 读取 CLI / N-API 启动页覆盖（不消费；不进 settings；无覆盖或空串时为 `None`）。
pub fn launch_override_screen() -> Option<String> {
    LAUNCH_OVERRIDE
        .lock()
        .expect("launch override lock")
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

/// 扁平字符串配置表（测试合并 / 遗留 TOML 扁平键）。
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
/// 优先级：`RA2_DIR` → `settings.json` 的 `ra2_dir` → 向上查找遗留 `RustAlert.toml` 的 `ra2_dir`。
/// 路径不存在则返回 `None`。**禁止**在调用方硬编码盘符或机主路径。
pub fn resolve_optional_install_root(search_from: &Path) -> Option<(PathBuf, Option<String>)> {
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

    let mut dir = if search_from.is_file() {
        search_from.parent().unwrap_or(search_from).to_path_buf()
    }
    else {
        search_from.to_path_buf()
    };
    loop {
        let cfg = dir.join(LEGACY_RUST_ALERT_TOML);
        if cfg.is_file() {
            if let Some(parsed) = parse_ra2_dir_from_toml_text(&std::fs::read_to_string(&cfg).ok()?) {
                let (root, edition) = parsed;
                if root.is_dir() {
                    return Some((root, edition));
                }
            }
        }
        if !dir.pop() {
            break;
        }
    }
    None
}

fn value_as_string(value: &Value) -> Option<String> {
    match value {
        Value::String(s) => Some(s.value().clone()),
        Value::Integer(i) => Some(i.to_string()),
        Value::Float(f) => Some(f.to_string()),
        Value::Boolean(b) => Some(b.to_string()),
        _ => None,
    }
}

/// 用 `toml_edit` 解析遗留文档根级键值为扁平表。
pub fn parse_toml_document(text: &str, source_label: &str) -> (ConfigTable, Vec<ConfigDiagnostic>) {
    let mut table = ConfigTable::new();
    let mut diagnostics = Vec::new();
    let doc: DocumentMut = match text.parse() {
        Ok(d) => d,
        Err(e) => {
            diagnostics.push(ConfigDiagnostic { source: source_label.into(), message: format!("TOML 解析失败: {e}") });
            return (table, diagnostics);
        }
    };
    for (key, item) in doc.iter() {
        match item {
            Item::Value(v) => match value_as_string(v) {
                Some(s) => table.insert(key, s),
                None => diagnostics.push(ConfigDiagnostic {
                    source: format!("{source_label}:{key}"),
                    message: format!("不支持的值类型，已跳过键 `{key}`"),
                }),
            },
            Item::None => {}
            Item::Table(_) | Item::ArrayOfTables(_) => {}
        }
    }
    (table, diagnostics)
}

/// 解析 0..1 音量；非法或非有限值返回 `None`。
pub fn parse_unit_volume(raw: &str) -> Option<f32> {
    let v: f32 = raw.trim().parse().ok()?;
    if v.is_finite() { Some(v.clamp(0.0, 1.0)) } else { None }
}
