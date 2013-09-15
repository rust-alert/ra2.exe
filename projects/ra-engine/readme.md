# ra-engine

一局游戏运行时：提交命令、按固定 tick 推进、取得呈现快照。不创建窗口、不初始化 GPU。

内部按 `runtime` / `state` / `spatial` / `gameplay` / `lifecycle` / `presentation` / `persistence` 划分。对外类型经本 crate 导出。

```shell
cargo test -p ra-engine
cargo test -p ra-testing
```

许可证：MPL-2.0。
