# ra-session

共享应用会话：持有 `World`、转发 `GameCommand`、按固定频率 `pump` / `tick`，并产出 `RenderSnapshot`。

遭遇战装载见 `open_skirmish_session`。不创建窗口、不初始化 GPU。

```shell
cargo test -p ra-session
```

跨 crate 无窗口回归在 `ra-testing`。遭遇战 `open_skirmish` 默认开启 AI：非本地阵营经同一 `GameCommand` 路径部署 MCV、放置电厂/兵营/战车工厂/矿场、生产步兵与载具，并对最近敌军自动攻击。`Session::new` 默认关闭 AI，便于单测。许可证：MPL-2.0。
