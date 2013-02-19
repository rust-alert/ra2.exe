# ra-webui

浏览器侧壳。`crate-type = ["cdylib", "rlib"]`，`publish = false`。描述写「Wasm / WebGL2 壳」—— **就现状而言，导出面只有三个
`wasm_bindgen` 函数**，画布、wgpu 表面、以及基于 `fetch` 的 `AssetSource` 都还没接上。

源码整文件：

```rust
//! 浏览器壳。画布与 fetch 版 AssetSource 后续再接。

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
```

## 三个导出分别做什么

| 导出              | 行为                                                                                                          |
|-------------------|---------------------------------------------------------------------------------------------------------------|
| `start`           | 标了 `#[wasm_bindgen(start)]`，模块加载时跑一次；只安装 `console_error_panic_hook`，让 panic 打到浏览器控制台 |
| `engine_name`     | 恒返回字符串 `"ra2"`，给页面侧识别引擎用                                                                      |
| `supports_webgl2` | 恒返回 `true`，**不探测**浏览器实际能力                                                                       |

没有创建 `<canvas>`，没有调用 `ra-renderer::Renderer`，也没有构造 `ra-world::World`。`Cargo.toml` 虽声明了 `ra-types` /
`ra-world` / `ra-renderer`，当前 `lib.rs` 未使用它们（为以后接线预留依赖）。工作区里的 `web-sys` 同样尚未列入本包依赖。

## 和 `ra-desktop` 差在哪

| 能力                               | `ra-desktop` | 本 crate     |
|------------------------------------|--------------|--------------|
| 读 `config.toml`                   | 有           | 无           |
| 探测安装目录 / 挂载 MIX            | 有           | 无           |
| winit 窗口 + wgpu 表面             | 有           | 无           |
| 每帧 `advance_tick` / `draw_frame` | 有           | 无           |
| Wasm 导出                          | 无           | 有上述三函数 |

要把浏览器目标做实，至少还需要：页面提供的 canvas、wgpu 的 WebGL2/WebGPU 适配路径、用 HTTP (S) 拉取或打包进站点的资源字节，以及实现
`AssetSource` 的 fetch/包装变体。这些都不在当前提交里。

## 构建说明

工具链见仓库根 `rust-toolchain.toml`（nightly）。本包是库，不是默认 `cargo run` 目标。常见做法是自行准备 `wasm32` 目标与打包工具（例如
`wasm-bindgen-cli` / `wasm-pack`），再由静态页加载生成的 `.wasm`。仓库内目前没有附带 `index.html` 或现成打包脚本——以你本地前端工程为准。

仅检查能否通过类型检查与本地编译：

```shell
cargo build -p ra-webui
```

交叉到 wasm 时请在本机安装对应 target 后再编；失败信息以 rustc / wasm-bindgen 输出为准。

## 依赖清单（manifest）

- `ra-types`、`ra-world`、`ra-renderer`（预留）
- `wasm-bindgen`
- `console_error_panic_hook`

无 feature 开关。

## 许可

MPL-2.0。浏览器部署同样要求使用者自行提供合法游戏数据来源；本仓库不附带原版 MIX / 音频 / 地图。

## 页面侧最小约定（在接线之前）

即便引擎侧尚未挂 canvas，页面已经可以：

1. 加载本包打出的 wasm
2. 调用 `engine_name()` 确认是 `"ra2"`
3. 读取 `supports_webgl2()`（今日恒 true，只能当占位，不能当能力检测）

真正初始化 GPU 时，应在 JS/TS 侧先检测 WebGL2（或未来 WebGPU），再决定是否调用引擎入口；不要依赖当前的 `supports_webgl2`
布尔值做生产判断。`start` 里的 panic hook 只影响 Rust panic 的可读性，不替代页面自己的错误 UI。

资源方面，桌面用本地目录 + MIX；浏览器需要另一套字节来源（例如用户选择的文件包、或站点托管的经授权资源）。设计 `AssetSource`
实现时保持与 `ra-types` 契约一致：`read(relative) -> RaResult<Vec<u8>>`，这样 `ra-rules` / `ra-map` 等解析路径可以复用，而不必为
Web 再写一套 INI/MIX 解析。

当前仓库也没有把 `ra-webui` 设成 `cargo run` 默认成员；日常原生开发走 `ra-desktop`。本包存在的意义是占住 Wasm
导出符号与依赖边，避免浏览器目标以后从零接线时再改工作区拓扑。
