# ra2

跨平台 GUI 引擎，用于运行《命令与征服：红色警戒 2》与《尤里的复仇》的自有资源。
玩家需自备正版游戏数据。本仓库不包含原版资源文件。

`ra-desktop` 构建原生二进制 **`ra2`**（Windows 上为 `ra2.exe`）。

## 配置

在工作目录放置 `config.toml`（或 `ra2.toml`）：

```toml
ra2_dir = "C:/path/to/your/ra2"
edition = "ra2"
```

`edition` 可为 `ra2` 或 `yr`；省略则按目录内容自动探测。

## 构建与运行

```shell
cargo run -p ra-desktop
```

浏览器目标（WebGL2）：`ra-web`。

## Crate

| Crate | 作用 |
|-------|------|
| `ra-types` | 基类型（`GameEdition`、错误、`AssetSource`） |
| `ra-assets` | MIX 等格式解析 |
| `ra-rules` | INI 规则投影 |
| `ra-map` | 地图 / 剧院 |
| `ra-world` | 确定性世界推进（`World`） |
| `ra-renderer` | wgpu 渲染（原生 + WebGL2） |
| `ra-desktop` | 原生 GUI 壳 → 二进制 `ra2` |
| `ra-web` | Wasm 壳 |
