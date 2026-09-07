# ra-engine

一局游戏运行时：提交命令、按固定 tick 推进、取得呈现快照。不创建窗口、不初始化 GPU。

当前内含原会话调度（`runtime`）；世界状态仍由 `ra-world` 提供。对外类型经本 crate 统一导出，[`Engine`](crate::Engine) 为对局句柄别名。

```shell
cargo test -p ra-engine
cargo test -p ra-testing
cargo test -p ra-session
```

许可证：MPL-2.0。
