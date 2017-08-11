//! 用户数据目录与规范文件名。

use std::path::{Path, PathBuf};
use std::sync::Mutex;

/// 产品用户数据目录名（各平台共用末级名）。
pub const APP_DATA_DIR_NAME: &str = "rust-alert2";

/// 配置文件名（config，非 state）。
pub const SETTINGS_FILE_NAME: &str = "settings.json";

/// 状态文件名（可变用户状态）。
pub const STATE_FILE_NAME: &str = "state.json";

/// Web `localStorage` 键前缀。
pub const WEB_STORAGE_PREFIX: &str = "rust-alert2.";

static TEST_USER_DATA_DIR: Mutex<Option<PathBuf>> = Mutex::new(None);

/// 测试用：覆盖 [`user_data_dir`]；传入 `None` 清除。
pub fn set_test_user_data_dir(path: Option<PathBuf>) {
    *TEST_USER_DATA_DIR.lock().expect("test user data dir lock") = path;
}

/// 当前可执行文件所在目录；失败时回退为 `"."`。
pub fn exe_dir() -> PathBuf {
    std::env::current_exe().ok().and_then(|p| p.parent().map(|d| d.to_path_buf())).unwrap_or_else(|| PathBuf::from("."))
}

/// 用户数据根目录（settings / state 落盘处）。
///
/// - 测试覆盖 / 环境变量 `RUST_ALERT2_DATA_DIR` 优先
/// - Windows：`%LOCALAPPDATA%/rust-alert2`
/// - macOS：`~/Library/Application Support/rust-alert2`
/// - 其它类 Unix：`$XDG_DATA_HOME/rust-alert2` 或 `~/.local/share/rust-alert2`
/// - wasm：逻辑目录名（真实读写走 localStorage，见 [`crate::store`]）
pub fn user_data_dir() -> PathBuf {
    if let Some(over) = TEST_USER_DATA_DIR.lock().expect("test user data dir lock").clone() {
        return over;
    }
    if let Ok(dir) = std::env::var("RUST_ALERT2_DATA_DIR") {
        let trimmed = dir.trim();
        if !trimmed.is_empty() {
            return PathBuf::from(trimmed);
        }
    }
    native_user_data_dir()
}

fn native_user_data_dir() -> PathBuf {
    #[cfg(target_os = "windows")]
    {
        if let Ok(local) = std::env::var("LOCALAPPDATA") {
            let trimmed = local.trim();
            if !trimmed.is_empty() {
                return PathBuf::from(trimmed).join(APP_DATA_DIR_NAME);
            }
        }
        return home_dir().unwrap_or_else(|| PathBuf::from(".")).join("AppData").join("Local").join(APP_DATA_DIR_NAME);
    }
    #[cfg(target_os = "macos")]
    {
        return home_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join("Library")
            .join("Application Support")
            .join(APP_DATA_DIR_NAME);
    }
    #[cfg(not(any(target_os = "windows", target_os = "macos")))]
    {
        if let Ok(xdg) = std::env::var("XDG_DATA_HOME") {
            let trimmed = xdg.trim();
            if !trimmed.is_empty() {
                return PathBuf::from(trimmed).join(APP_DATA_DIR_NAME);
            }
        }
        return home_dir().unwrap_or_else(|| PathBuf::from(".")).join(".local").join("share").join(APP_DATA_DIR_NAME);
    }
}

fn home_dir() -> Option<PathBuf> {
    std::env::var_os("HOME")
        .or_else(|| std::env::var_os("USERPROFILE"))
        .map(PathBuf::from)
}

/// `settings.json` 规范路径。
pub fn settings_path() -> PathBuf {
    user_data_dir().join(SETTINGS_FILE_NAME)
}

/// `state.json` 规范路径。
pub fn state_path() -> PathBuf {
    user_data_dir().join(STATE_FILE_NAME)
}

/// 确保用户数据目录存在。
pub fn ensure_user_data_dir() -> Result<PathBuf, String> {
    let dir = user_data_dir();
    std::fs::create_dir_all(&dir).map_err(|e| format!("创建用户数据目录失败 ({}): {e}", dir.display()))?;
    Ok(dir)
}

/// 相对用户数据根的子路径。
pub fn user_data_join(name: impl AsRef<Path>) -> PathBuf {
    user_data_dir().join(name)
}
