//! 持久化后端：桌面写文件，Web 写 `localStorage`。

use crate::paths::{SETTINGS_FILE_NAME, STATE_FILE_NAME, ensure_user_data_dir, user_data_dir};

#[cfg(target_arch = "wasm32")]
use crate::paths::WEB_STORAGE_PREFIX;

/// 文本槽位读写（settings / state）。
pub trait PersistStore: Send + Sync {
    /// 读取命名槽；不存在则 `Ok(None)`。
    fn read_text(&self, name: &str) -> Result<Option<String>, String>;
    /// 写入命名槽。
    fn write_text(&self, name: &str, text: &str) -> Result<(), String>;
}

/// 当前平台默认 store。
pub fn default_store() -> Box<dyn PersistStore> {
    #[cfg(target_arch = "wasm32")]
    {
        Box::new(LocalStorageStore)
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        Box::new(NativeDirStore)
    }
}

/// 桌面：用户数据目录下的 JSON 文件。
#[derive(Debug, Default, Clone, Copy)]
pub struct NativeDirStore;

impl PersistStore for NativeDirStore {
    fn read_text(&self, name: &str) -> Result<Option<String>, String> {
        let path = user_data_dir().join(name);
        if !path.is_file() {
            return Ok(None);
        }
        let text = std::fs::read_to_string(&path).map_err(|e| format!("读取 {} 失败: {e}", path.display()))?;
        Ok(Some(text))
    }

    fn write_text(&self, name: &str, text: &str) -> Result<(), String> {
        ensure_user_data_dir()?;
        let path = user_data_dir().join(name);
        std::fs::write(&path, text).map_err(|e| format!("写入 {} 失败: {e}", path.display()))
    }
}

/// Web：`localStorage` 键 `rust-alert2.settings` / `rust-alert2.state`。
#[derive(Debug, Default, Clone, Copy)]
pub struct LocalStorageStore;

#[cfg(target_arch = "wasm32")]
impl PersistStore for LocalStorageStore {
    fn read_text(&self, name: &str) -> Result<Option<String>, String> {
        let key = storage_key(name);
        let storage = window_local_storage()?;
        match storage.get_item(&key) {
            Ok(v) => Ok(v),
            Err(_) => Err(format!("localStorage 读取失败: {key}")),
        }
    }

    fn write_text(&self, name: &str, text: &str) -> Result<(), String> {
        let key = storage_key(name);
        let storage = window_local_storage()?;
        storage.set_item(&key, text).map_err(|_| format!("localStorage 写入失败: {key}"))
    }
}

#[cfg(not(target_arch = "wasm32"))]
impl PersistStore for LocalStorageStore {
    fn read_text(&self, _name: &str) -> Result<Option<String>, String> {
        Err("LocalStorageStore 仅用于 wasm 目标".into())
    }

    fn write_text(&self, _name: &str, _text: &str) -> Result<(), String> {
        Err("LocalStorageStore 仅用于 wasm 目标".into())
    }
}

#[cfg(target_arch = "wasm32")]
fn storage_key(name: &str) -> String {
    let stem = name.strip_suffix(".json").unwrap_or(name);
    format!("{WEB_STORAGE_PREFIX}{stem}")
}

#[cfg(target_arch = "wasm32")]
fn window_local_storage() -> Result<web_sys::Storage, String> {
    let window = web_sys::window().ok_or_else(|| "无 Window".to_string())?;
    window.local_storage().map_err(|_| "localStorage 不可用".to_string())?.ok_or_else(|| "localStorage 为 None".to_string())
}

/// 读 settings 槽。
pub fn read_settings_text(store: &dyn PersistStore) -> Result<Option<String>, String> {
    store.read_text(SETTINGS_FILE_NAME)
}

/// 写 settings 槽。
pub fn write_settings_text(store: &dyn PersistStore, text: &str) -> Result<(), String> {
    store.write_text(SETTINGS_FILE_NAME, text)
}

/// 读 state 槽。
pub fn read_state_text(store: &dyn PersistStore) -> Result<Option<String>, String> {
    store.read_text(STATE_FILE_NAME)
}

/// 写 state 槽。
pub fn write_state_text(store: &dyn PersistStore, text: &str) -> Result<(), String> {
    store.write_text(STATE_FILE_NAME, text)
}

/// 调试用：当前 settings / state 落点说明。
pub fn persist_location_label() -> String {
    #[cfg(target_arch = "wasm32")]
    {
        format!("localStorage `{WEB_STORAGE_PREFIX}settings` / `{WEB_STORAGE_PREFIX}state`")
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        format!("{} · {}", crate::paths::settings_path().display(), crate::paths::state_path().display())
    }
}
