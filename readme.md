# ra2

跨平台 GUI 引擎，用于在玩家自备的《命令与征服：红色警戒 2》、《尤里的复仇》以及心灵终结 3（Mental Omega 3）数据上运行自有逻辑。

本仓库是 **现代化重写**：产品入口为 npm 包 **`@game-gpt/red-alert2`**（CLI `ra2 launch --path`），经 N-API 拉起 `ra-desktop` 宿主库；对局由 **`ra-engine`** 推进，呈现走现代 GPU API。**不是** DirectDraw 兼容层，**不是**向原版 `game.exe` / `gamemd.exe` 注入。

仓库 **不包含**原版 MIX / INI / 音频 / 地图等资源文件。运行前请自行准备合法取得的游戏安装目录。

---

## 安装与启动

```bash
npm i -g @game-gpt/red-alert2
ra2 launch --path "C:/Games/RA2"
ra2 launch --path "C:/Games/RA2" --edition yr
```

`--path` 指向含零售 MIX/INI 的安装根目录。

---

## 开发构建

```shell
pnpm install
pnpm run build
pnpm exec ra2 --help

cargo test -p ra-assets -p ra-map -p ra-engine -p ra-testing
cargo run -p ra-desktop --example launch
cargo run -p ra-desktop --example probe_boot -- "C:/path/to/your/ra2"

pnpm run lint
pnpm run fmt
```

配置模板：`RustAlert.toml.example`。CLI `--path` 优先于其中的 `ra2_dir`。

---

## 启动链路

```text
ra2 launch --path
    -> packages/red-alert2
    -> bindings/ra-napi
    -> hosts/ra-desktop
    -> ra-engine + ra-renderer + ra-adaptor
```

浏览器路径（后续）：`sites/playground` -> `packages/red-alert2-wasm` -> `bindings/ra-wasm` -> 同一套引擎库。

---

## 仓库布局

```text
projects/
  engine/       # ra-types ... ra-net
  adapters/     # ra-adaptor*
  hosts/        # ra-desktop
  bindings/     # ra-napi, ra-wasm
  tooling/      # ra-testing (later ra-modder)
  packages/     # @game-gpt/red-alert2
  platforms/native/
```

| package / crate | role |
|------------|------|
| `@game-gpt/red-alert2` | CLI + Node API |
| `@game-gpt/red-alert2-<os-cpu>` | native N-API addon |
| `ra-desktop` | host window / input / event loop |
| `ra-napi` | Node binding |
| `ra-wasm` | browser binding |
| `ra-engine` | authoritative match state |
| `ra-testing` | test support |

---

## 许可

见 [`License.md`](License.md)（Apache-2.0）。
