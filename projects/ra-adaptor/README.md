# ra-adaptor

按 `GameEdition` 装配资源表，并探测安装布局。整个 crate 只有一个 `src/lib.rs`，没有子模块。

依赖：`ra-types`、`ra-adaptor-ra2`、`ra-adaptor-yr`。原版 / YR 的文件名清单不写在这里，而在两个 profile crate；本层做编排与磁盘探测。

---

## 探测状态机：`detect_edition`

```text
root 不是目录？ → Io("游戏目录不存在: …")
有显式 GameEdition？ → 直接用
否则：
  looks_like(YR) × looks_like(RA2)
  (true, false)  → Ra2
  (false, true)  → Yr
  (true, true)   → AmbiguousEdition
  (false, false) → CannotDetectEdition
然后：
  ResourceChain::for_edition(edition)
  扫描 chain.root_mix_files → present_mixes / missing_mixes
  打成 EditionManifest
```

显式版本优先于启发式。混装目录（既有 `game.exe` 又有 `gamemd.exe`）必须在 `config.toml` 写明 `edition`，否则启动失败——这是刻意行为，不是漏判。

`scan_root_mixes` 只用 `find_ci_file` 看文件在不在， **不打开 MIX、不验索引**。嵌套包名列在 `chain.nested_mix_files`
里，挂载发生在 `ra-desktop` 的 `boot_world`，不在本 crate。

---

## `ResourceChain`

统一视图，字段全部是 `'static` 字符串切片引用：

| 字段                                             | 含义                                       |
|--------------------------------------------------|--------------------------------------------|
| `edition`                                        | `GameEdition`                              |
| `root_mix_files`                                 | 安装根旁主 MIX                             |
| `nested_mix_files`                               | 主 MIX 内常见嵌套名                        |
| `rules_ini` / `art_ini` / `ui_ini` / `sound_ini` | INI 文件名                                 |
| `exe_name`                                       | 布局特征用的主程序名（引擎不启动原版 exe） |

`ResourceChain::for_edition` 内部 `match`：

- `Ra2` → `from_ra2(ra_adaptor_ra2::profile())`
- `Yr` → `from_yr(ra_adaptor_yr::profile())`
- `Mo3` → `from_mo3(ra_adaptor_mo3::profile())`

`from_ra2` / `from_yr` / `from_mo3` 是私有映射函数，把各 edition 的 `ResourceProfile` 抄进同一结构。profile 类型故意重复定义（注释：避免跨
crate 循环依赖），所以映射不能写成泛型一份。

`ra-rules::load_rules` 只通过 `ResourceChain::for_edition` 取 `rules_ini` / `art_ini`，不自己写 `rulesmd.ini`。

---

## `EditionManifest`

```rust
pub struct EditionManifest {
    pub root: PathBuf,
    pub chain: ResourceChain,
    pub present_mixes: Vec<String>,
    pub missing_mixes: Vec<String>,
}
```

桌面启动日志里的「缺盘 N」来自 `missing_mixes.len()`；真正 `mount_bytes` 时用的是 `present_mixes` 里能再次 `find_ci_file`
到的路径。缺盘不一定阻止开窗——那是壳层策略，本层只报告。

---

## `find_ci_file`

大小写不敏感查找，返回 **实际磁盘路径**：

1. 先试 `root.join(wanted)` 是否为文件
2. 否则 `read_dir`，把每个名字 ASCII 小写后与目标比较
3. 命中且 `is_file` 则返回该 `PathBuf`

Windows 默认不敏感，但开发机、网络盘、将来非 Windows 目标可能不同；资源表里的名字大小写固定，磁盘上却可能是 `RA2.MIX`。
`GameAssetSource` 读松散文件时也走同一助手。

---

## 与 profile crate 的分工

| Crate            | 职责                                            |
|------------------|-------------------------------------------------|
| `ra-adaptor-ra2` | 原版静态表 + `looks_like`                       |
| `ra-adaptor-yr`  | YR 静态表 + `looks_like`                        |
| `ra-adaptor-mo3` | 心灵终结 3 静态表 + `looks_like`                |
| **本 crate**     | 消歧、装配 `ResourceChain`、扫描根 MIX、CI 查找 |

本层是 adaptor 家族里 **唯一直接 `std::fs`** 的（`is_dir` / `read_dir` / `is_file`）。仍然不解析内容。

探测时若命中心灵终结 3 启发式，优先 `Mo3`，不再与原版/YR 报歧义。

---

## 构建与调用

```shell
cargo build -p ra-adaptor
```

无测试、无 feature。真实目录验证请跑 `ra-desktop` 或它的 `examples/probe_*.rs`。

典型调用（桌面）：

```text
DesktopConfig → GameEdition::parse(可选)
→ detect_edition(root, explicit)
→ 对 present_mixes 做 mount_bytes
→ 对 nested_mix_files 做 mount_nested
→ load_rules / 读地图 …
```

许可证 MPL-2.0。玩家自备游戏目录。本仓库不含 MIX / INI 二进制。
