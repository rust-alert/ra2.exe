# ra-adaptor-mo3

心灵终结 3（Mental Omega 3）的磁盘旁与启动期资源表。结构与 `ra-adaptor-yr` 对称：自有一份 `ResourceProfile`，只回答「去哪些文件名找字节」。

依赖仅 `ra-types`。本 crate **不是** Un4seen `.mo3` 音乐容器解析器。

---

## 布局要点

心灵终结 3 **不是**独立零售盘：需要合法取得的原版 + 尤里的复仇基座文件，再叠 MO 自身的 `expandmo*` / `mapsmo03.mix` 等。

引擎共内核：仿真 / 渲染不散落 `if mo3 { … }`，应拿 `ResourceChain` 上的字段。

---

## `profile()` 摘要

### 根 MIX

- 基座：`language.mix` / `langmd.mix` / `ra2.mix` / `ra2md.mix` / `multimd.mix` / `thememd.mix`
- MO：`expandmo95/96/97/99.mix`、`mapsmo03.mix`、`multimo.mix`、`movmo03.mix`
- 可选：`expandmo98.mix`（语言包）、`expandmo94.mix` / `thememo.mix`（原声）

### 嵌套 MIX

与 YR 同形的 `*md.mix` 与 `expandmd0*`。

### INI / exe

| 字段        | 值                           |
|-------------|------------------------------|
| `rules_ini` | `rulesmd.ini`                |
| `art_ini`   | `artmd.ini`                  |
| `ui_ini`    | `uimd.ini`                   |
| `sound_ini` | `soundmd.ini`                |
| `exe_name`  | `MentalOmegaClient.exe`      |
| `edition`   | `GameEdition::Mo3`           |

---

## `looks_like`

```text
MentalOmegaClient.exe || RA2MO.ini || expandmo99.mix || expandmo97.mix || mapsmo03.mix
```

与 YR/RA2 合集目录并存时，探测优先认 MO；也可配置：

```toml
edition = "mo3"
```

别名：`mo` / `mentalomega` / `mental-omega` / `mental_omega`。

---

## 引擎如何选中 MO3 chain

1. 配置 `edition = "mo3"`（或上表别名）→ `GameEdition::Mo3`
2. 或自动探测命中本 crate 的启发式（优先于纯 YR）
3. `ResourceChain::for_edition(Mo3)` → `from_mo3(profile())`
4. `load_rules` 读 `rulesmd.ini` / `artmd.ini`
5. 桌面挂载上表根包与嵌套包
