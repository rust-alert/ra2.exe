//! 浏览器壳。画布与 fetch 版 `AssetSource` 后续再接。

use wasm_bindgen::prelude::*;

#[wasm_bindgen(start)]
pub fn start() {
    console_error_panic_hook::set_once();
}

#[wasm_bindgen]
pub fn engine_name() -> String {
    "ra2".into()
}

#[wasm_bindgen]
pub fn supports_webgl2() -> bool {
    true
}
