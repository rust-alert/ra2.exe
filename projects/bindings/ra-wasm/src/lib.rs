//! Wasm 绑定。产物进入 `platforms/wasm/red-alert2-unknown-wasm32`，再由 `@game-gpt/red-alert2` 整合。
//!
//! 宿主胶水在 [`host`]；与 `ra-napi` 同构职责，页面不进本 crate。

#![deny(missing_docs)]

pub mod host;

pub use host::install::InstallSession;

use wasm_bindgen::prelude::*;

/// Wasm 模块加载后的入口钩子。
#[wasm_bindgen(start)]
pub fn start() {
    console_error_panic_hook::set_once();
}

/// 绑定版本字符串（与 `ra-napi::version` 对齐）。
#[wasm_bindgen]
pub fn version() -> String {
    env!("CARGO_PKG_VERSION").to_string()
}

/// 引擎显示名（占位）。
#[wasm_bindgen]
pub fn engine_name() -> String {
    "ra2".into()
}

/// 运行时是否已具备可用的 wgpu 呈现路径（画布 / surface / adapter）。
///
/// 未接线前恒为 `false`。浏览器后端当前多为 WebGL2，但这是 wgpu 的呈现能力探测，不是手写 GL 绑定。
#[wasm_bindgen]
pub fn supports_present() -> bool {
    false
}
