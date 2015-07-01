# `@game-gpt/red-alert2`

现代化重写的《红色警戒 2》CLI。自行准备合法游戏安装目录后启动：

```bash
npm i -g @game-gpt/red-alert2
ra2 launch --path "C:/Games/RA2"
```

可选：`--edition ra2|yr`、`--screen skirmish`（跳过闪屏直达遭遇战大厅等）。

```bash
ra2 launch --path "C:/Games/RA2" --edition ra2 --screen skirmish
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
