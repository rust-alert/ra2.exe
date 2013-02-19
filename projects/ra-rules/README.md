# ra-rules

打开 `src/lib.rs`：有效逻辑不到三十行。crate 描述叫「INI 规则投影」，今天实际做的是—— **按版本选出 rules/art 文件名，经
`AssetSource` 读字节，解析成两份 `IniDocument`，塞进 `RulesDb`。**

没有单位表索引，没有武器伤害换算，没有把 `Prerequisite=` 变成图。`World::new` 目前还把传入的 `&RulesDb` 直接
`let _ = rules` 丢掉。所以本 README 不写「规则引擎架构」，只写装载器真实行为。

## 唯一结构体与唯一函数

```rust
pub struct RulesDb {
    pub edition: GameEdition,
    pub rules: IniDocument,
    pub art: IniDocument,
}

pub fn load_rules(
    source: &dyn AssetSource,
    edition: GameEdition,
) -> RaResult<RulesDb>
```

体内顺序：

1. `let chain = ResourceChain::for_edition(edition);`
2. `source.read(chain.rules_ini)` → `IniDocument::parse`
3. `source.read(chain.art_ini)` → `IniDocument::parse`
4. 组装 `RulesDb { edition, rules, art }`

原版会读到 `rules.ini` / `art.ini`；尤里的复仇会读到 `rulesmd.ini` / `artmd.ini`。这些名字来自 adaptor 表，本文件里没有字符串字面量。

## 故意没加载的东西

`ResourceChain` 上还有 `ui_ini`、`sound_ini`。本 crate **不读它们**。UI 字符串与音效表若要投影，要么扩展 `RulesDb`，要么另起
crate——现在两者都没有。

## 依赖为什么长这样

```text
ra-types     → AssetSource / GameEdition / RaResult
ra-adaptor   → ResourceChain::for_edition
ra-assets    → IniDocument::parse
```

也可以说：本 crate 的存在价值就是把「选文件名」和「解析 INI」粘在一处，让 `ra-world` / `ra-desktop` 不必各自重复这两行。薄，但是边界清。

## 谁在调用

`ra-desktop` 启动流水线在挂载 MIX、尝试读启动地图之后调用 `load_rules`；成功则 `World::new(edition, &rules, map)`
。失败会进入桌面的错误路径（世界可能为空，窗口仍可能打开——见桌面 README）。

`ra-desktop/examples/probe_boot.rs` 也会 `load_rules` 并打印节数量之类的诊断信息，适合在无 GUI 时验证安装。

## 名实差距与演进时注意点

叫「投影」是预留语义：将来应在 `RulesDb` 上提供只读查询（按单位名取节、把数值解析成 `Fixed16` 等），而不是让每个系统自己
`rules.section("xxx").get("Strength")`。在那之前：

- 改玩家目录里的 INI **不会**自动变成可见战场行为（世界还没用规则）
- 但启动期仍应加载，以便尽早暴露缺文件 / 解析错误
- 缺键默认值策略（零？继承？报错？）一旦引入查询 API，必须写进函数文档并测住

## 测试

本 crate 无 `#[cfg(test)]`。要用假数据测装载，可自写实现 `AssetSource` 的内存源，对 `ra2`/`yr` 两种 edition 分别喂最小
INI。不要把完整零售 `rules.ini` 提交进仓库。

## 错误会从哪冒出来

| 阶段                          | 典型结果                                                                 |
|-------------------------------|--------------------------------------------------------------------------|
| `source.read(rules_ini)` 失败 | `MissingFile` / `Io`（取决于 `AssetSource` 实现）                        |
| rules 文本不是合法 INI 结构   | `IniDocument::parse` → `Parse`                                           |
| art 同样                      | 同上；rules 已成功也不会提交半成品 `RulesDb`（函数内顺序执行，中途 `?`） |

桌面在规则失败时仍可能开窗（`world = None`），因此「能看见窗口」不等于规则已加载。看标题 / 标准错误里的 `rules#` 或「规则待加载」更准。

## `IniDocument` 能做什么、不能做什么

本包不重新实现 INI，只用 `ra-assets::IniDocument`：

- 能：按节名/键名 `get`，遍历 `sections`（桌面用 `rules.sections.len()` 打进 boot note）
- 不能：类型化单位定义、继承展开、`Prerequisite` 图、对艺术帧序列的语义解释

那些属于「投影做实」之后的 API。在那之前，其它 crate 若临时 `rules.get("SOMEUNIT", "Strength")`，请把缺键策略写在调用点，并预期日后迁到
`RulesDb` 方法上。

## 构建与许可

```shell
cargo build -p ra-rules
```

MPL-2.0。读的是用户目录里的规则文本；不要把完整零售 `rules.ini` / `rulesmd.ini` 提交进 git。若要落地查询 API，建议先加方法与测试，再让
`World::new` 真正消费，避免世界层先散落字符串解析。
