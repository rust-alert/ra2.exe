//! N-API 入口：`version` + `launch({ path, edition? })`。

#![deny(clippy::all)]

use std::path::PathBuf;

use napi::bindgen_prelude::*;
use napi_derive::napi;
use ra_config::LaunchOverride;

/// 绑定版本字符串。
#[napi]
pub fn version() -> String {
    env!("CARGO_PKG_VERSION").to_string()
}

/// `ra2 launch` 选项。
#[napi(object)]
pub struct LaunchOptions {
    /// 游戏安装根目录（含 MIX / INI）。
    pub path: String,
    /// 可选版本：`ra2` / `yr` 等。
    pub edition: Option<String>,
}

/// 注入路径覆盖并阻塞进入 GUI 事件循环。
#[napi]
pub fn launch(options: LaunchOptions) -> Result<()> {
    let path = PathBuf::from(options.path.trim());
    if path.as_os_str().is_empty() {
        return Err(Error::from_reason("--path must not be empty"));
    }
    ra_config::set_launch_override(LaunchOverride { ra2_dir: path, edition: options.edition.filter(|s| !s.trim().is_empty()) });
    ra_desktop::run().map_err(|e| Error::from_reason(format!("{e}")))
}
