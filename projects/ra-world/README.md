# ra-world

整份实现就在 `src/lib.rs`。crate 注释两句：

> 确定性世界推进。不依赖渲染器与文件系统。

没有子系统目录，没有实体表，没有寻路。当前「世界」四个公开字段加一个私有哈希，外加三个方法。

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

注意哈希 **还没**混入地图尺寸、单元格、实体或规则。它现在的作用是：证明「有一条可复现的状态摘要管道」，并给桌面窗口标题提供变化的
`t{N}`。多机锁步若直接拿今天的 `state_hash` 当校验，信息量不够——那是未来要把更多字段折进哈希时的事。

### `state_hash(&self) -> u64`

只读访问器。

## 谁每帧调用它

`ra-desktop` 在 `WindowEvent::RedrawRequested` 里：

1. `world.advance_tick()`（若 world 存在）
2. `renderer.draw_frame(world.as_ref())`
3. 刷新标题 `… · t{tick}`
4. `request_redraw()`，事件循环 `ControlFlow::Poll`

也就是说：tick 目前跟绘制帧绑在一起，不是独立的固定赫兹仿真时钟。若以后要 15/30 Hz 逻辑帧，应把 `advance_tick` 从 redraw
里拆出去——那是桌面壳的调度问题，本结构体仍然只负责「推进一次」。

## 和渲染器的边界

`ra-renderer::draw_frame` 接收 `Option<&World>`，但实现里对世界的使用极其克制（edition 相关逻辑基本可忽略）。渲染器有 GPU
副作用；世界刻意保持纯状态，方便以后在无窗口环境跑确定性测试。不要把 `wgpu` 类型引进本 crate。

## 还没接上的基类型

`ra-types` 里的 `Fixed16`、`EntityId`、`TypeId` 本应首先在世界层落地。现状：只有 `PlayerId` 出现。README 不假装已经有单位列表。

## 构建

```shell
cargo build -p ra-world
```

无测试、无 feature。许可 MPL-2.0。

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
