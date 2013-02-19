# ra-adaptor-mo3

MO3 音乐载荷适配。工作区里体积最小的 adaptor：一个 `lib.rs`，三个公开项， **目前没有被其它 crate 引用**。

源码模块注释原文：

> 零售主题曲常见为 MIX 内嵌音频；部分发行或模组会使用 MO3。  
> 本 crate 先暴露探测与占位解码接口，不阻塞引擎启动。

也就是说：桌面启动热路径不依赖本 crate；没有 MO3 也能开窗。

---

## 公开表面（完整）

### `looks_like_mo3(data: &[u8]) -> bool`

前三个字节是否为 `M` `O` `3`，且 `len >= 3`。不做更深校验。

### `Mo3Track`

```rust
pub struct Mo3Track {
    pub byte_len: usize,
}
```

注释：「解码结果占位：后续可换成 PCM 样本或交给音频后端。」现在只有长度。

### `probe(data: &[u8]) -> RaResult<Mo3Track>`

- 魔数不对 → `Err(RaError::Parse("不是 MO3 载荷"))`
- 否则 → `Ok(Mo3Track { byte_len: data.len() })`

**不展开样本、不解码、不分配 PCM 缓冲。**

---

## 和主题音乐路径的关系（当前）

`ra-adaptor-ra2` / `yr` 的资源表里有 `theme.mix` / `thememd.mix`。那些是 MIX 档案名，由 `MixVfs` 挂载。MO3
是另一种容器，识别逻辑单独放在这里，避免把音频格式细节塞进安装布局表。

音频播放、混音、淡入淡出、任务曲切换——都不在本 crate，也不在当前 `ra-desktop` 主循环里。

---

## 依赖与构建

```toml
# Cargo.toml
description = "MO3 音乐载荷适配"
# dependencies: ra-types
```

```shell
cargo build -p ra-adaptor-mo3
```

无单元测试、无 feature、无 example。若你在上层接线，建议至少补：

- 真魔数短切片 → `looks_like_mo3 == true`
- 空切片 / 随机头 → false
- `probe` 错误文案稳定

测试夹具用合成字节即可， **不要**把受版权保护的原版主题曲提交进仓库。

---

## 若以后要变成真解码器

合理边界（与现有分层一致）：

1. 壳或资源层用 `AssetSource` / VFS 拿出字节
2. `looks_like_mo3` / `probe` 识别
3. 本 crate（或专用解码 crate）产出 PCM 或中间表示
4. 桌面 / Web 壳对接平台音频 API
5. `ra-world` 只发「播放曲目」类命令，不碰设备

`looks_like_mo3` 必须保持 O (1) 头检查，不要扫完整文件。Wasm 目标还要考虑内存上限。

在解码落地之前，保持「可编译、API 诚实、不拖启动」即可。根目录 README 的 crate 表已收录本包：适配接口存在，播放未接。

## 调用示例

```rust
use ra_adaptor_mo3::{looks_like_mo3, probe};

fn inspect(buf: &[u8]) {
    if !looks_like_mo3(buf) {
        return;
    }
    match probe(buf) {
        Ok(track) => println!("mo3 bytes={}", track.byte_len),
        Err(e) => eprintln!("{e}"),
    }
}
```

把这段嵌进资源扫描器时，建议先按扩展名或所在目录缩小候选集，再对命中文件读头几个字节做 `looks_like_mo3`，避免对每个 MIX
内条目都做全量分配。`probe` 目前几乎不比魔数检查多做什么，但错误类型已统一到 `RaResult`，上层可以用同一套错误展示。

## 与 `theme.mix` 的并存策略

零售安装里主题音乐往往在 `theme.mix` / `thememd.mix` 内，走 MIX + 既有音频格式。MO3 更常见于特定发行包装或模组替换曲。并行存在时：

- 布局表继续负责挂载 `theme*.mix`
- 本 crate 只在「拿到一段疑似 MO3 的字节」时介入
- 播放列表（曲目 ID、循环、淡入）应放在更高层数据，而不是写进魔数检测

这样原版路径与 MO3 路径可以逐步接线，而不会在 `ra-adaptor` 的版本探测里引入音频分支。

## 许可

MPL-2.0。本包不含音乐样本文件。

## 头字节边界情况

| 输入                          | `looks_like_mo3`                    | `probe`               |
|-------------------------------|-------------------------------------|-----------------------|
| `b"MO3"`                      | true                                | Ok，`byte_len = 3`    |
| `b"MO3...."` 更长载荷         | true                                | Ok，`byte_len = 全长` |
| `b"MO"` / 空切片              | false                               | Err Parse             |
| `b"mo3"` 小写                 | false（逐字节比的是大写 `M``O``3`） | Err                   |
| 前三字节碰巧是 MO3 的其它格式 | true（假阳性可能）                  | Ok，仅长度            |

因为只认三字节魔数， **不能**把 `looks_like_mo3 == true` 当成「一定可播放」。真正解码器落地后，应在 `probe` 或后续 API
里校验更多头字段，并把失败从「不是 MO3」细分为「版本不支持」「损坏」等。在那之前，上层应把本 API 当成粗过滤器。
