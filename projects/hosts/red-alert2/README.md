# `@game-gpt/red-alert2`

现代化重写的《红色警戒 2》CLI。自行准备合法游戏安装目录后启动：

```bash
npm i -g @game-gpt/red-alert2
ra2 emulate --path "C:/Games/RA2"
```

可选：`--edition ra2|yr`、`--screen skirmish`（跳过闪屏直达遭遇战大厅等）。

```bash
ra2 emulate --path "C:/Games/RA2" --edition ra2 --screen skirmish
```

按逻辑名导出资源：

```bash
ra2 extract --path "C:/Games/RA2" --out ./out --decode-shp -- sdtp.shp title.pcx
```

全量解包已挂载 MIX 树（按档案分子目录；原名由内置哈希恢复表还原，未命中则 `id_XXXXXXXX.bin`）：

```bash
ra2 unpack --path "C:/Games/RA2" --out ./unpacked
ra2 unpack --path "C:/Games/RA2" --out ./unpacked --names-file ./extra_names.txt
```

分析类命令（`ra2 analyze <topic>`）：

遭遇战地图包装载／准备／能力缺口三态（`success` / `reject` / `missing`；不是整局可玩验收）：

```bash
ra2 analyze maps --path "C:/Games/RA2" --edition ra2
ra2 analyze maps --path "C:/Games/RA2" --edition ra2 --limit 5
ra2 analyze maps --path "C:/Games/RA2" --edition ra2 --json
```

`--json` 输出与 N-API `diagnoseMaps` 相同的报告对象，便于回填验收清单。

载具 VXL 分图层（车身／炮塔／炮管／合成／落影的尺寸与原点偏移；只读安装资源，不写盘）：

```bash
ra2 analyze vxl --path "C:/Games/RA2" --edition ra2 --stem mtnk
ra2 analyze vxl --path "C:/Games/RA2" --edition ra2 --type MTNK --sweep-turret
ra2 analyze vxl --path "C:/Games/RA2" --edition ra2 --stem mtnk --sweep-hva --hva-frames 3 --json
```

合集盘须加 `--edition ra2`。`--sweep-body` / `--sweep-turret` / `--sweep-hva` 三者互斥。
