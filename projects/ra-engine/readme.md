# ra-engine

一局游戏运行时入口：提交命令、按固定 tick 推进、取得呈现快照。不创建窗口、不初始化 GPU。

过渡期本包转发 `ra-session` 与 `ra-world` 的公开类型；[`Engine`](crate::Engine) 为对局句柄别名。消费方应优先依赖本 crate。

```shell
cargo test -p ra-engine
cargo test -p ra-testing
```

许可证：MPL-2.0。
