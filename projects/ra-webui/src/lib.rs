//! 浏览器壳。画布与 fetch 版 `AssetSource` 后续再接。

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

/// 是否声明支持 WebGL2 目标（占位，尚未接线画布）。
#[wasm_bindgen]
pub fn supports_webgl2() -> bool {
    true
}
