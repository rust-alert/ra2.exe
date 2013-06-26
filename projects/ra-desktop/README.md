# ra-desktop

工作区默认成员。产出原生 GUI 二进制 **`ra2`**（Windows 上为 `ra2.exe`）。`publish = false`。

这不是命令行工具：没有子命令解析栈。进程读工作目录旁的配置，跑完启动流水线后把控制权交给 winit 窗口。Release 构建在 Windows
上使用 `windows_subsystem = "windows"`（无控制台窗口）；Debug 仍可看到 `eprintln!` 诊断。

## 目录里有什么

```
src/main.rs       入口、boot、App 事件循环、预览选取
src/config.rs     委托 `ra-config` 加载桌面设置
src/fs_source.rs  GameAssetSource：松散文件优先，再查 MixVfs
examples/
  probe_boot.rs     无窗口：挂载 + load_rules
  probe_tmp.rs      无窗口：剧院 TMP → 不透明像素统计
  probe_shp.rs      无窗口：SHP 首帧 + unittem.pal
  probe_theater.rs  无窗口：剧院 MIX / pal / 地图体积
```

`Cargo.toml` 未声明 `[[example]]`；用 Cargo 跑示例时仍可：

```shell
cargo run -p ra-desktop --example probe_boot -- path/to/game
```

（若本地 Cargo 认不到 example，把文件路径显式配进清单即可。）

## 配置

按顺序尝试读取当前工作目录下的 `config.toml`、`ra2.toml`。解析器是手写「一行一个 `key = value`」，不是完整 TOML 库：

| 键                      | 作用                                                                  |
|-------------------------|-----------------------------------------------------------------------|
| `ra2_dir` 或 `game_dir` | 含零售 MIX / INI 的目录                                               |
| `edition`               | 可选；`ra2` / `yr` / `mo3` 及 `GameEdition::parse` 接受的别名；省略则自动探测 |

`#` 之后当注释；以 `[` 开头的行跳过。缺文件时默认 `ra2_dir = "."`、`edition = None`。

示例：

```toml
ra2_dir = "C:/Games/RA2"
edition = "ra2"
```

运行：

```shell
cargo run -p ra-desktop
# 或在工作区根（default-members 已是本包）
cargo run
```

需要自行准备合法游戏数据目录；仓库不附带原版资源。

## 启动流水线（`boot_world`）

顺序与 `main.rs` 一致：

1. `detect_edition(root, explicit)` → `EditionManifest`
2. 对 `present_mixes`：`find_ci_file` → `fs::read` → `MixVfs::mount_bytes`
3. 对 `chain.nested_mix_files`：`mount_nested`（失败/缺失静默跳过）
4. `load_boot_map`：候选 `mp01t4.map` / `mp01t2.map` / `mp02t4.map`；成功则按剧院 `theater_mix_names` 再 `mount_nested`
5. 预览图优先级：
    - `load_map_terrain_preview`：若 `map.cells` 非空，用剧院调色板 + tileset + TMP，经 `compose_terrain_rgba` 拼整图
    - 否则 `load_preview_terrain`：单砖候选（tileset 槽 0/14/9/10/12，再 `clear01.{ext}`）
    - 再否则 `load_preview_sprite`：`unittem.pal` + `mouse.shp` / `e1.shp` / `clock.shp` / `power.shp` / `gaairc.shp`
6. `load_rules` → 成功则 `World::new`；失败则 `world = None`，`note` 里写「规则待加载」
7. 组装 `BootResult { note, world, preview }`

探测整段失败时，`run` 仍会开窗，只是 `note` 变成 `启动失败: …`，`world`/`preview` 为空——便于看见 GPU 与窗口路径是否正常。

## 窗口标题里的 `boot_note`

标题格式：`ra2 ({edition}) · {boot_note} · t{tick}`。

`boot_note` 典型片段（按出现拼接）：

- `{edition} · 根mix N · 嵌套 N · 跳过 N · 缺盘 N`
- `map:{name} WxH theater · 剧院mix N` 或 `map:无`
- 若 Iso 单元非空：`iso#N`
- `preview:…` 或 `preview:无`
- `rules#节数` 或规则错误摘要

标准错误输出还会打一行 `ra2 boot: … · world=ok|none`，以及 GPU 附着后的 `ra2 gpu: {backend} · preview=yes|no`。

## `GameAssetSource`

实现 `AssetSource::read`：

1. 安装根上 `find_ci_file`（大小写不敏感）→ 直接读磁盘
2. 否则 `vfs.read`
3. 都没有 → `MissingFile`

因此开发时可把某个 `.shp` / `.ini` 放在游戏目录旁覆盖包内同名条目，而无需改 MIX。

## 事件循环

`App` 实现 winit `ApplicationHandler`：

- `resumed`：创建约 1024×768 窗口，`renderer.attach_window`
- `RedrawRequested`：`world.advance_tick()`（若有）→ `draw_frame` → 刷新标题 → 再 `request_redraw`
- `about_to_wait`：再次 `request_redraw`
- `ControlFlow::Poll`

当前逻辑帧与绘制帧绑在一起，不是独立赫兹的仿真时钟。

## 依赖面

串联工作区几乎全部库：`ra-types`、`ra-adaptor`、`ra-assets`、`ra-config`、`ra-map`、`ra-world`、`ra-session`、`ra-renderer`，外加 `winit`。不直接依赖
`ra-adaptor-phobos`（经 `ra-adaptor` 间接装配）、`ra-webui`。

## 许可

MPL-2.0（与工作区一致）。二进制只是引擎壳；玩法数据来自用户指定的目录。
