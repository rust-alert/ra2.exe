# ra-wasm

Rust → 浏览器绑定。`scripts/build/wasm.mjs` 产出到 `platforms/wasm/red-alert2-unknown-wasm32/pkg`，再由 `@game-gpt/red-alert2` 对外整合。

当前 `0.0.0` 占位只导出 `engine_name` / `supports_webgl2`（恒 `false`），保证 CI 能编过真实 `.wasm`。引擎接线后续再加。

站点（homepage / playground）只依赖已发布的 `@game-gpt/red-alert2`，不要直接依赖本平台包。
