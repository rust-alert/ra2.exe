# ra-napi

N-API 绑定 + 原生宿主（`projects/bindings/ra-napi`）。供 npm 包 **`@game-gpt/red-alert2`** 在 Node 侧加载，对外暴露 `version` / `launch` / `extract` / `unpack`。

本 crate 产出 `cdylib` + `rlib`。原生窗口、输入与事件循环在 `host/`；UI 布局与组件分别在 `ra-layout` / `ra-components`。

```mermaid
flowchart LR
    A["ra2 launch --path"] --> B[ra-napi]
    B --> C["host::run"]
    B --> D[extract / unpack]
    C --> E[ra-components]
    C --> F[ra-layout]
```

## 构建

```shell
pnpm run build:napi
```

产物由 `scripts/build/napi.mjs` 写入平台包目录，再由 `@game-gpt/red-alert2` 整合。

原生壳与资源工具统一走 TypeScript CLI（不要再用 `cargo run` / Rust `bin` / `example`）：

```shell
pnpm exec ra2 launch --path "C:/Games/RA2" --edition ra2
```

## 调用约定

合集盘 / 混装安装目录启动或导出资源时须显式指定版本，例如：

```shell
pnpm exec ra2 launch --path "C:/Games/RA2" --edition ra2
pnpm exec ra2 extract --path "C:/Games/RA2" --edition ra2 --out ./tmp/extract -- sdtp.shp
pnpm exec ra2 unpack --path "C:/Games/RA2" --edition ra2 --out ./tmp/unpack
```

省略 `--edition` 时，适配层可能落到资料片资源链。
