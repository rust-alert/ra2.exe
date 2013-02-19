# ra-adaptor-ra2

原版红色警戒 2 的 **磁盘旁与启动期资源表**。一个 `lib.rs`，没有子模块，几乎没有逻辑——差异优先当数据。

依赖仅 `ra-types`。消费方是 `ra-adaptor`（映射成 `ResourceChain`），不是桌面直接依赖。

---

## `ResourceProfile` 字段

```rust
pub struct ResourceProfile {
    pub edition: GameEdition,              // 固定 Ra2
    pub root_mix_files: &'static [&'static str],
    pub nested_mix_files: &'static [&'static str],
    pub rules_ini: &'static str,
    pub art_ini: &'static str,
    pub ui_ini: &'static str,
    pub sound_ini: &'static str,
    pub exe_name: &'static str,
}
```

注释：「某一版本期望的文件清单（差异优先当数据）。」根 MIX 用大小写不敏感匹配；嵌套 MIX「位于主 MIX 内，启动后按需挂载」。

结构与 `ra-adaptor-yr::ResourceProfile` **同形但各自定义**，避免 adaptor 编排层与两边形成循环依赖。

---

## `profile()` 字面量清单

### 根目录 MIX（`root_mix_files`）

按源码顺序：

| 文件名         | 在启动链中的角色（简述）  |
|----------------|---------------------------|
| `language.mix` | 语言 / 本地化相关主包之一 |
| `ra2.mix`      | 原版主内容包              |
| `multi.mix`    | 多人相关                  |
| `theme.mix`    | 主题 / 音乐相关           |
| `maps01.mix`   | 地图包 1                  |
| `maps02.mix`   | 地图包 2                  |

`detect_edition` 只扫描这些名字是否在安装根存在，填入 `present_mixes` / `missing_mixes`。

### 嵌套 MIX（`nested_mix_files`）

| 文件名        |
|---------------|
| `local.mix`   |
| `cache.mix`   |
| `conquer.mix` |
| `generic.mix` |
| `isogen.mix`  |
| `cameo.mix`   |
| `audio.mix`   |

桌面 `boot_world` 在根包 `mount_bytes` 成功后，对这一列表逐个 `MixVfs::mount_nested`。挂载顺序影响「先命中者优先」的
overlay（见 `ra-assets::MixVfs`）。

### INI 与 exe

| 字段        | 值          |
|-------------|-------------|
| `rules_ini` | `rules.ini` |
| `art_ini`   | `art.ini`   |
| `ui_ini`    | `ui.ini`    |
| `sound_ini` | `sound.ini` |
| `exe_name`  | `game.exe`  |

当前 `ra-rules::load_rules` 只读 rules + art。`ui.ini` / `sound.ini` 已在表里，加载器尚未用。`exe_name` 是安装布局特征名；引擎是自有
GUI（`ra2` / `ra2.exe`）， **不注入、不启动**原版 `game.exe`。

---

## `looks_like`

```rust
root.join("game.exe").is_file()
 || root.join("rules.ini").is_file()
 || root.join("ra2.mix").is_file()
 || root.join("language.mix").is_file()
```

任意一条为真即判「像原版」。这是 OR 启发式，不是完整性证明：

- 只剩一个残缺 `language.mix` 也可能判真
- 与 YR 目录混装时，两边都可能真 → `ra-adaptor` 报 `AmbiguousEdition`
- 大小写：这里用的是 `Path::join` 直接拼接，在大小写敏感文件系统上，磁盘若是 `RA2.MIX` 而这里写 `ra2.mix`，可能判假；真正挂载阶段会再用
  `find_ci_file`

自动探测不可靠时，在配置里写 `edition = "ra2"`。

---

## 和 YR 表怎么对照

不要在本文件复制 YR 列表。对照时打开 `ra-adaptor-yr`：那边是 `*md*` 命名，嵌套多了 `expandmd01`–`03`，INI 带 `md`，exe 是
`gamemd.exe`。改原版表时问自己：YR 是否也有对称项？两边 `looks_like` 是否仍能分开？

---

## 构建

```shell
cargo build -p ra-adaptor-ra2
```

无测试、无 feature。改 `profile()` 后，请在真实原版目录上跑桌面启动或探针，确认根 MIX 扫描与嵌套挂载计数仍合理。

许可证 MPL-2.0。表里是文件名，不是资源内容；原版 MIX 不得进入本仓库 git 历史。
