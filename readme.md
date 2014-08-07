# ra2

跨平台 GUI 引擎，用于在玩家自备的《命令与征服：红色警戒 2》、《尤里的复仇》以及心灵终结 3（Mental Omega 3）数据上运行自有逻辑。

本仓库是 **现代化重写**：产品入口为 npm 包 **`@game-gpt/red-alert2`**（CLI `ra2 launch --path`），经 N-API 拉起原生窗口实现 `ra-desktop`；对局由 **`ra-engine`** 推进，呈现走现代 GPU API。**不是** DirectDraw 兼容层，**不是**向原版 `game.exe` / `gamemd.exe` 注入。

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

# 按逻辑名导出 / 全量解包（含哈希原名恢复）
pnpm exec ra2 extract --path "C:/Games/RA2" --out ./tmp/extract --decode-shp -- sdtp.shp title.pcx
pnpm exec ra2 unpack --path "C:/Games/RA2" --out ./tmp/unpack
pnpm exec ra2 unpack --path "C:/Games/RA2" --out ./tmp/unpack --names-file ./extra_names.txt

pnpm run lint
pnpm run fmt
```

配置模板：`RustAlert.toml.example`。CLI `--path` 优先于其中的 `ra2_dir`。

---

## 启动链路

```text
ra2 launch --path
    -> hosts/red-alert2              # @game-gpt/red-alert2
    -> bindings/ra-napi
    -> bindings/ra-desktop          # 原生窗口 / 输入 / 事件循环
    -> ra-engine + ra-renderer + ra-adaptor
```

浏览器路径（后续）：`sites/playground` -> `@game-gpt/red-alert2`（`./wasm`）-> `platforms/wasm/red-alert2-unknown-wasm32` <- `bindings/ra-wasm`。

---

## 仓库布局

```text
projects/
  engine/       # ra-types ... ra-net
  adapters/     # ra-adaptor*
  bindings/     # ra-napi, ra-wasm, ra-desktop
  tooling/      # ra-testing（后续 ra-modder）
  hosts/        # red-alert2（@game-gpt/red-alert2）
  platforms/
    native/     # @game-gpt/red-alert2-<os-cpu>
    wasm/       # @game-gpt/red-alert2-unknown-wasm32
```

| package / crate | role |
|------------|------|
| `@game-gpt/red-alert2` | CLI + Node API + 浏览器面 |
| `@game-gpt/red-alert2-<os-cpu>` | 原生 N-API 插件 |
| `@game-gpt/red-alert2-unknown-wasm32` | Wasm 产物平台包 |
| `ra-desktop` | 原生窗口 / 输入 / 事件循环（仅供 `ra-napi`） |
| `ra-napi` | Node 绑定 |
| `ra-wasm` | 浏览器绑定 |
| `ra-engine` | 权威对局状态 |
| `ra-testing` | 测试支持 |

---

## 许可

见 [`License.md`](License.md)（Apache-2.0）。
