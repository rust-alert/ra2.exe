# ra-types

全体 `ra-*` crate 共享的基类型。源码就五个模块，没有解析器，没有窗口，没有 `std::fs`。

```
src/
  lib.rs          重导出
  edition.rs      GameEdition
  error.rs        RaError / RaResult
  ids.rs          EntityId / PlayerId / TypeId
  fixed.rs        Fixed16
  asset_source.rs AssetSource
```

`Cargo.toml` 描述为「RA 引擎基类型」。依赖 `thiserror`（给 `RaError`）与工作区里的 `bitflags`（清单已声明，源码尚未使用）。

---

## 类型目录

### `GameEdition`

```rust
pub enum GameEdition { Ra2, Yr }
```

注释写明：原版对应 `game.exe` / `rules.ini`，尤里的复仇对应 `gamemd.exe` / `rulesmd.ini`。这是「选哪一套零售规则与资源」的枚举，不是模组
ID。

| 方法     | 行为                           |
|----------|--------------------------------|
| `as_str` | `Ra2` → `"ra2"`，`Yr` → `"yr"` |
| `parse`  | 见下表                         |

`parse` 接受的别名（先 `trim` 再 ASCII 小写）：

| 输入                           | 结果                       |
|--------------------------------|----------------------------|
| `ra2` / `vanilla` / `original` | `Ra2`                      |
| `yr` / `yuri` / `yuris` / `md` | `Yr`                       |
| 其它                           | `Err(UnknownEdition(...))` |

配置文件里的 `edition` 字段最终会走到这里；写错字符串会在启动早期失败，而不是默默当成原版。

### `RaError` / `RaResult`

`RaResult<T> = Result<T, RaError>`。变体与 `thiserror` 文案：

| 变体                  | 典型触发                                              |
|-----------------------|-------------------------------------------------------|
| `UnknownEdition`      | `GameEdition::parse` 吃到怪字符串                     |
| `AmbiguousEdition`    | 目录同时像原版又像 YR（`ra-adaptor::detect_edition`） |
| `CannotDetectEdition` | 两边启发式都不命中                                    |
| `MissingFile`         | 缺必要文件                                            |
| `Parse`               | MIX / INI / 地图等解析失败                            |
| `Io`                  | 读写失败（壳层常把 `std::io` 转进来）                 |
| `Msg`                 | 通用消息                                              |

共享错误枚举的意义：桌面壳、规则加载、格式解析可以共用 `?`，不必在边界上反复 `map_err`。

### 标识符：`EntityId` / `PlayerId` / `TypeId`

三个 newtype，分别包 `u64` / `u8` / `u32`，都派生 `Debug/Clone/Copy/Eq/Ord/Hash/Default`。字段公开（`pub` 元组结构体），没有额外方法。
`ra-world::World` 目前只用了 `PlayerId(0)` 作为 `local_player`；`EntityId` / `TypeId` 是给后续实体与规则类型索引预留的。

### `Fixed16`

16.16 定点，存在 `i32` 里。模块注释写的是「确定性世界推进用的定点数辅助」。

| 项                                  | 含义            |
|-------------------------------------|-----------------|
| `ZERO`                              | `0`             |
| `ONE`                               | `1 << 16`       |
| `from_i32`                          | 左移 16         |
| `to_i32_trunc`                      | 右移 16（截断） |
| `saturating_add` / `saturating_sub` | 饱和加减        |

没有乘除、没有从浮点构造。世界层也还没真正用它推进单位坐标——放在基库是为了以后各子系统引用同一表示，而不是半套 `f32`、半套整数。

### `AssetSource`

```rust
pub trait AssetSource {
    fn read(&self, relative: &str) -> RaResult<Vec<u8>>;
    fn exists(&self, relative: &str) -> bool { self.read(relative).is_ok() }
}
```

模块注释：「平台 I/O 边界：由壳实现；解析器只看见字节。」`ra-assets` 刻意不碰文件系统；`ra-rules::load_rules` 只收
`&dyn AssetSource`。桌面侧的实现是 `ra-desktop` 里的 `GameAssetSource`（松散文件优先，再查 `MixVfs`）。

默认 `exists` 会整文件 `read` 一次再丢结果——简单，但对大文件不经济。需要 `stat` 优化时，由具体实现覆盖该方法即可。

---

## 刻意不放进本 crate 的东西

- MIX / SHP / TMP / INI 解析 → `ra-assets`
- 安装目录探测 → `ra-adaptor`
- 规则语义 → `ra-rules`（当前也只是 INI 投影）
- 地图 IsoMapPack → `ra-map`
- GPU → `ra-renderer`

基类型一旦开始读盘或创建窗口，原生壳与 Wasm 壳就无法共用同一套契约。

---

## 构建

工作区根目录有 `rust-toolchain.toml`（nightly）。在仓库根：

```shell
cargo build -p ra-types
```

无 feature、无 bin、无测试模块。改公共类型等于改全工作区 ABI 约定，提交前应用 `cargo build` 把下游 crate 编过一遍。

---

## 谁依赖谁

几乎所有 `ra-*` 都依赖本 crate。典型消费方式：

1. 配置字符串 → `GameEdition::parse`
2. 加载失败 → `RaError` 向上冒泡
3. 读资源 → 实现或传入 `AssetSource`
4. 世界状态 → 将来用 `Fixed16` / `EntityId` 等

许可证为工作区统一的 **MPL-2.0**。本仓库不含原版游戏资源；运行需要自行提供合法取得的游戏数据。
