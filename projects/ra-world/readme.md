# ra-world

整份实现就在 `src/lib.rs`。crate 注释两句：

> 确定性世界推进。不依赖渲染器与文件系统。

当前包含实体、命令帧、寻路和最小战斗系统。它不创建窗口、不初始化 GPU、不读取安装目录。外层 `ra-session` 负责固定 tick 调度，adaptor 负责提供已加载的 `RulesDb`。

## 结构体长什么样

```rust
pub struct World {
    pub edition: GameEdition,
    pub tick: u64,
    pub map: MapInfo,
    pub local_player: PlayerId,  // new 时固定 PlayerId(0)
    state_hash: u64,             // 私有
}
```

依赖：`ra-types`、`ra-adaptor`、`ra-assets`、`ra-map`。`World::new` 消费 `RulesDb`（来自 `ra-adaptor`）绑定 Techno 数值与地图实体。
——规则内容此刻不进入状态。

## 三个方法的真实语义

### `new(edition, rules, map)`

- `tick = 0`
- `state_hash = 0`
- `local_player = PlayerId(0)`
- `map` 原样持有（可能是桌面解析成功的图，也可能是 `MapInfo::empty`）

### `advance_tick`

```text
tick = tick.wrapping_add(1)
state_hash = state_hash
    .wrapping_mul(1099511628211)   // FNV-1a 常用质数
    .wrapping_add(tick)
    .wrapping_add(edition.as_str().len() as u64)
```

哈希当前混入地图尺寸、实体状态、目标、冷却、路径和最近输入帧，可用于本地确定性回归。它仍不代表完整零售玩法状态，不能单独作为既有战网兼容证明。

### `state_hash(&self) -> u64`

只读访问器。

## 谁调用它

`ra-session` 通过 `pump` 按 15Hz 固定步长调用 `advance_tick`。桌面重绘只生成快照并呈现，不决定逻辑 tick 数量。测试可直接调用 `Session::tick` 精确单步。

## 和渲染器的边界

`ra-renderer::draw_frame` 接收 `Option<&RenderSnapshot>`。渲染器有 GPU
副作用；世界刻意保持纯状态，方便在无窗口环境跑确定性测试。不要把 `wgpu` 类型引进本 crate。

## 当前边界

`GameCommand` 现含 `MoveTo`、`Attack`、`Deploy`、`PlaceBuilding`、`Produce` 与 `SetRallyPoint`。实体已带稳定 `EntityId`。建造链支持建造场/供电前置、资金扣除与矿场周期入账。工厂单槽生产队列在 `PRODUCE_TICKS` 后出厂，并可按集结点自动寻路。攻击射程/伤害/冷却优先取自 rules 主武器节（`Primary` → `Range`/`Damage`/`ROF`），否则回退到 `Sight`、`Strength/4` 与预览常量。

## 构建

```shell
cargo build -p ra-world
```

跨 crate 的无窗口遭遇战回归在 `ra-testing`，许可 MPL-2.0。

## `state_hash` 手算示例

初始：`tick = 0`，`state_hash = 0`。第一次 `advance_tick`：

1. `tick` 变为 `1`
2. `state_hash = 0 * 1099511628211 + 1 + len("ra2"|"yr")`

`GameEdition::as_str()` 长度：`ra2` 为 3，`yr` 为 2。因此同一套代码、不同 edition，从第一跳开始哈希就会分叉——即便地图与规则完全未参与。这是刻意把
edition 编进摘要的最小做法；也说明今天的哈希 **不能**用来验证「两边是否加载了同一张图」。

若你在测试里断言哈希，请固定 edition，并只依赖「连续调用 N 次后的值」，不要假设混入了 `MapInfo::cells`。

## 读源码时的预期

当前定位是： **带 tick / 摘要骨架的世界句柄**，还不是完整战场仿真。合理的后续增量包括：实体存贮、规则查询结果变成组件、把地图关键字段折进哈希、把逻辑
tick 从 redraw 解耦。每改哈希输入集合，请同步改本节公式说明，免得读者仍以为「只混 tick 与 edition 字符串长度」。

本 crate 不读盘、不创建窗口；单测可以在无 GPU 环境直接 `World::new` + 循环 `advance_tick`。

## 字段在集成时的用法

- `edition`：窗口标题、以及将来按版本分支的数据查询（尽量少用分支，优先数据驱动）。
- `tick`：桌面标题 `t{N}`；调试「是否在推进」。
- `map`：持有启动时解析的 `MapInfo`（可能 `cells` 为空）。渲染地形预览在桌面侧读的是同一份 map，世界本身不绘制。
- `local_player`：固定 `PlayerId(0)`，尚未接多玩家或热座。
- `state_hash()`：只读；外部不要假设它稳定跨版本——哈希输入集合变更时数值会变。

构造失败模式：本类型的 `new` 本身不返回 `Result`；失败发生在更外层的地图/规则加载。传入的 `map` 可以是 `MapInfo::empty`
，世界仍然可创建并推进 tick。
