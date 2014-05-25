//! Wasm 绑定。画布与 fetch 版 `AssetSource` 后续再接。
//! 平台 npm 包：`@game-gpt/red-alert2-unknown-wasm32`。

#![deny(missing_docs)]

use wasm_bindgen::prelude::*;

/// Wasm 模块加载后的入口钩子。
#[wasm_bindgen(start)]
pub fn start() {
    console_error_panic_hook::set_once();
}

/// 引擎显示名（占位）。
#[wasm_bindgen]
pub fn engine_name() -> String {
    "ra2".into()
}

/// 运行时是否已具备可用的 WebGL2 呈现路径。
///
/// 画布与 adapter 未接线前恒为 `false`，禁止把本函数当成「目标平台声明」或 JS 侧探测的替代。
#[wasm_bindgen]
pub fn supports_webgl2() -> bool {
    false
}
