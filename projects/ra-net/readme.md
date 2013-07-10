# ra-net

联机协议的基础类型 crate。当前提供对局标识、内容指纹、带玩家编号和 tick 的输入消息、状态摘要及同步消息枚举。

```shell
cargo check -p ra-net
```

当前提供：

- 消息类型与 `PROTOCOL_VERSION` / `MAX_PAYLOAD_BYTES`
- 长度前缀成帧：`encode_frame` / `decode_frame`
- 严格顺序序号窗：`SequenceWindow`
- 内容指纹：`MatchFingerprint::build`

尚未实现传输、房间、锁步调度、重连或观战。平台壳负责传输与编排，世界负责验证和执行游戏命令；本 crate 不依赖窗口、文件系统或 GPU。

许可证：MPL-2.0。
