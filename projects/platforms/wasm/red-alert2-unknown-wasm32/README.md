# `@game-gpt/red-alert2-unknown-wasm32`

由 Rust crate `ra-wasm` 产出的 Wasm 平台包。站点与浏览器侧 **不要** 直接依赖本包；统一接入 `@game-gpt/red-alert2`（`./wasm`）。

导出：`init` / `version` / `engineName` / `supportsPresent` / `attachCanvas` / `resizePresent` / `presentFrame` / `presentBackend` / `resumeAudio` / `audioReady` / `audioState` / `InstallSession` / `PrepareReport` / `loadJobBusy` / `loadJobProgress`。
