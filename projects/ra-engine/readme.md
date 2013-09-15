# ra-engine

一局游戏运行时：提交命令、按固定 tick 推进、取得呈现快照。不创建窗口、不初始化 GPU。

内含权威世界（`world`）与对局调度（`runtime`）。对外类型经本 crate 统一导出，[`Engine`](crate::Engine) 为对局句柄别名。`ra-session` / `ra-world` 为过渡期兼容转发。

```shell
cargo test -p ra-engine
cargo test -p ra-testing
cargo test -p ra-world
cargo test -p ra-session
```

许可证：MPL-2.0。
