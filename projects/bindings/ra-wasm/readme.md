# ra-wasm

Rust → 浏览器绑定 + 宿主胶水（`projects/bindings/ra-wasm`）。职责与 `ra-napi` 同构：画布 / 输入 / 装载 / 音频副作用；**不**承载页面组合（页面在 `ra-widgets`）。

`scripts/build/wasm.mjs` 产出到 `platforms/wasm/red-alert2-unknown-wasm32/pkg`，再由 `@game-gpt/red-alert2` 整合。浏览器产品对标面是 `sites/playground`，不是本 crate。

当前导出 `version` / `engine_name` / `supports_present` / `attachCanvas` / `presentBackend`，以及 `InstallSession` / `PrepareReport` / `loadJobBusy` / `loadJobProgress`。画布经 wgpu 附着后 `supports_present` 为真。

站点（homepage / playground）只依赖已发布的 `@game-gpt/red-alert2`，不要直接依赖本平台包。
