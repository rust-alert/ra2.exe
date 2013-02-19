# ra-adaptor-yr

尤里的复仇的磁盘旁与启动期资源表。结构与 `ra-adaptor-ra2` 对称，但 **全部文件名走 `md` 体系**，嵌套列表更长。

依赖仅 `ra-types`。`lib.rs` 内自有一份 `ResourceProfile`（注释写明与原版同形，为避免跨 crate 循环依赖而重复定义）。

---

## 命名规律

零售布局里，资料片文件名相对原版常加 `md`：

| 角色         | 原版（对照）    | 本 crate          |
|--------------|-----------------|-------------------|
| 语言包       | `language.mix`  | `langmd.mix`      |
| 主包         | `ra2.mix`       | `ra2md.mix`       |
| 多人         | `multi.mix`     | `multimd.mix`     |
| 主题         | `theme.mix`     | `thememd.mix`     |
| 地图         | `maps01/02.mix` | `mapsmd01/02.mix` |
| 规则         | `rules.ini`     | `rulesmd.ini`     |
| 主程序特征名 | `game.exe`      | `gamemd.exe`      |

引擎共内核：仿真 / 渲染不该散落 `if yr { "rulesmd.ini" }`，应拿 `ResourceChain` 上的字段。本 crate 的职责就是把这些字符串集中成数据。

---

## `profile()` 完整列表

### 根 MIX

```text
langmd.mix
ra2md.mix
multimd.mix
thememd.mix
mapsmd01.mix
mapsmd02.mix
```

### 嵌套 MIX

相对原版多出三段扩展包：

```text
localmd.mix
cachemd.mix
conqmd.mix      （注意不是 conquermd）
genermd.mix
isogenmd.mix
cameomd.mix
audiomd.mix
expandmd01.mix
expandmd02.mix
expandmd03.mix
```

`conqmd` / `genermd` 是源码里的真实拼写，对照原版 `conquer.mix` / `generic.mix` 时不要想当然补全字母。挂载顺序按数组顺序；
`MixVfs` 先挂载先命中。

### INI / exe

| 字段        | 值                |
|-------------|-------------------|
| `rules_ini` | `rulesmd.ini`     |
| `art_ini`   | `artmd.ini`       |
| `ui_ini`    | `uimd.ini`        |
| `sound_ini` | `soundmd.ini`     |
| `exe_name`  | `gamemd.exe`      |
| `edition`   | `GameEdition::Yr` |

---

## `looks_like`

```rust
gamemd.exe || rulesmd.ini || ra2md.mix || langmd.mix
```

任一存在即真。与原版启发式对称。合集盘或「原版目录里再塞一套 YR 文件」时，两边 `looks_like` 可能同时为真，此时必须：

```toml
edition = "yr"
```

否则 `detect_edition` 返回 `AmbiguousEdition`。

本函数用 `Path::join` 直接拼小写名，不走 `find_ci_file`；大小写敏感盘上若实际文件名大小写不同，启发式可能偏保守。挂载阶段仍会
CI 查找。

---

## 引擎如何选中 YR chain

1. 配置 `edition = "yr"`（或别名 `yuri` / `yuris` / `md`）→ `GameEdition::Yr`
2. 或自动探测仅 YR 特征命中
3. `ResourceChain::for_edition(Yr)` → `from_yr(profile())`
4. `load_rules` 读 `rulesmd.ini` / `artmd.ini`
5. 桌面挂载上表根包与嵌套包

资料片游戏性差异主要在玩家数据（INI / 图像），不在本表。本表只回答「去哪些文件名找字节」。

---

## 维护时成对审查

改 `expandmd*` 或嵌套顺序前：

1. 是否影响 overlay 命中（错误 SHP / 调色板先被读到）
2. `ra-adaptor-ra2` 是否需要对称说明（文档层面）
3. 真实 YR 目录上 `present_mixes` / `mount_nested` 计数是否合理

```shell
cargo build -p ra-adaptor-yr
```

无单元测试。表数据变更后，建议在真实尤里的复仇目录上观察桌面启动日志里的「根mix / 嵌套 / 缺盘」计数，以及 `rules#` 是否出现；比只
`cargo build` 更能发现拼写错误。

## 常见误判场景

- **合集目录**：原版与资料片文件放在同一文件夹 → 两边 `looks_like` 都可能为真 → 必须在配置写 `edition = "yr"`（或 `ra2`）。
- **只有扩展包**：只有 `expandmd01.mix` 而没有 `ra2md.mix` / `gamemd.exe` 等特征文件 → 本启发式可能为假，自动探测失败；显式
  `edition` 仍可装配 chain，但缺盘列表会很长。
- **模组改名**：把 `rulesmd.ini` 改成别的名字 → `looks_like` 可能仍因 `ra2md.mix` 判真，但 `load_rules` 会 `MissingFile`。

## 许可

MPL-2.0。本包只含文件名常量，不含 MIX / INI 内容。
