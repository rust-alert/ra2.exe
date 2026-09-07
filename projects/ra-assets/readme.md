# ra-assets

crate 根注释一句话定调： **格式解析：字节进、结构出。不碰 `std::fs`。**

这是工作区里算法最密的库。其它 crate 谈「挂哪个文件」；这里谈「字节长什么样」。依赖：`ra-types`、`blowfish`、`byteorder`、
`num-bigint`。

## 源文件树

```
src/lib.rs          再导出
src/mix_hash.rs     名称 → i32 哈希（CRC-32 + Westwood 填充）
src/mix_crypto.rs   RSA 解 Blowfish 钥，ECB 解索引
src/mix.rs          旧头 / 新格式 / 加密索引档案
src/mix_vfs.rs      多层挂载，先命中优先
src/ini.rs          节/键；编号键数值排序拼接
src/pal.rs          768 字节 VGA 调色板 → RGBA
src/shp/mod.rs      SHP(TS) 头与帧
src/shp/decode.rs   RLE-Zero 行解码
src/tmp.rs          TMP 等距地形砖（钻石像素展开）
```

公开再导出：`IniDocument`/`IniSection`、`MixArchive`/`MixEntry`、`mix_hash`、`MixVfs`、`Palette`/`Rgba`、`ShpFile`/`ShpFrame`、
`TmpFile`/`TmpTile`。`mix_crypto` 默认当内部实现用。

---

## MIX 名称哈希（`mix_hash.rs`）

- 多项式 `0xEDB88320`，编译期生成 `CRC32_TABLE[256]`
- 流程：名字转大写 → `westwood_pad`（长度不是 4 的倍数时，先塞 `residue as u8`，再重复某位置字符填齐）→ CRC32 → 转成 `i32`
- 测试锁住：标准向量 `crc32(b"123456789") == 0xCBF43926`；`rules.ini` 大小写同哈希；`RULES.INI` 填充后长度 12 且特定字节位置符合约定

`MixArchive::get_by_name` 走的就是这个哈希，再在按 id 排序的条目上二分。

---

## 索引解密（`mix_crypto.rs`）

注释写明：算法与公开工具 ccmixar / ccmix 同族； **只保护索引，正文不加密**。

关键常量：

| 名                       | 值                                                       |
|--------------------------|----------------------------------------------------------|
| `RSA_EXPONENT`           | 65537                                                    |
| `RSA_MODULUS_BE`         | 40 字节（注释：源自 Westwood `keys.ini` 公开 Base64 钥） |
| `RSA_KEY_BLOCK_SIZE`     | 80                                                       |
| `BLOWFISH_KEY_SIZE`      | 56                                                       |
| `BLOWFISH_BLOCK_SIZE`    | 8                                                        |
| `RSA_COMBINE_SHIFT_BITS` | 312                                                      |

`extract_blowfish_key`：反转 80 字节 → 两半 RSA `modpow` → `(s0 << 312) + s1` → 取出 56 字节再 reverse。  
`blowfish_decrypt_ecb`：`Blowfish<BE>`，输入长度必须是 8 的倍数。

单元测试 `blowfish_roundtrip_aligned` 用对齐块做加密再解密往返。

---

## `MixArchive`（`mix.rs`）

常量：`FLAG_ENCRYPTED = 0x0002`，新格式头 4 字节，索引头 6 字节，条目 12 字节。

判别：首 `u16 == 0` → 新格式（再看 flags 是否加密）；否则旧格式且首字当 file_count。加密路径走上面的 RSA/Blowfish 解索引。
`MixEntry { id, offset, size }`。

测试 `parse_old_header_single_entry`：旧头单条目能按 id 读出正文。

---

## `MixVfs`（`mix_vfs.rs`）

`mount` / `mount_bytes` / `mount_nested` / `read`。多层档案叠在一起， **先挂载的先命中**。`read` 返回字节拷贝。桌面先挂根包再
`mount_nested` 表内嵌套名；剧院 MIX 也是后来 `mount_nested`。

本类型仍然不碰磁盘：谁读文件谁传入 `Vec<u8>`。

---

## INI（`ini.rs`）

`;` 行注释，`[section]`，`key=value`。内部保留插入顺序，供地图 IsoMapPack 拼接。`numbered_section_concat`：把节里数字键按
**数值**排序后拼接值（测试：`1=A,2=B,10=C` → `"ABC"`，不会被字典序弄成 `1,10,2`）。

`ra-adaptor`（装载 `RulesDb`）与 `ra-map` 都吃这个解析器。另导出 `ColorSchemes` / `OverlayTypeRegistry` /
`TechnoTypeRegistry` 等 INI 派生表。

---

## PAL（`pal.rs`）

必须恰好 768 字节。组件按 VGA 6-bit 左移 2 位扩到 8-bit。透明：索引 0，或 raw RGB 为品红键 `[63,0,63]`。`to_rgba_bytes()` →
1024 字节。测试覆盖尺寸拒绝、位移、品红透明。

---

## SHP（`shp/`）

SHP (TS)：首字须 0；文件头 8 字节 + 每帧 24 字节描述。`FORMAT_RLE_ZERO_BIT = 0x02`。RLE：每行前缀 `u16` 长度；非零字面量；
`0, count` 表示零行程。`ShpFrame::to_rgba(&Palette)` 供桌面启动预览。测试含 1 像素 raw、基本 RLE 行、全透明行。

---

## TMP（`tmp.rs`）——地形砖，不是「临时文件」

注释：TMP 等距地形砖：模板头 + 钻石像素展开。

公开类型：

- `TmpFile`：`template_width/height`、`tile_width/height`、`tiles: Vec<Option<TmpTile>>`
- `TmpTile`：高度、地形类型、坡道类型、矩形像素缓冲（钻石外为 0）、宽高与偏移

头大小 16；每砖描述 52 字节。标志位 `HAS_EXTRA_DATA` / `HAS_Z_DATA`。钻石展开用 `DIAMOND_INITIAL_WIDTH = 4`、
`DIAMOND_WIDTH_STEP = 4`。桌面示例 `probe_tmp` 可在自备游戏目录上验证这些结构；渲染器尚未批量画 TMP。

---

## 怎么跑测试

```shell
cargo test -p ra-assets
```

几乎所有格式相关回归都集中在这里。夹具用合成字节，不要往 git 里塞原版 MIX。

## 使用纪律

1. 禁止在本 crate 加 `std::fs`「图省事」——一加，Wasm 目标就被绑死。
2. 大包整包进内存在浏览器可能炸；VFS 调用方要控制挂载数量。
3. 名称查找必须用本库 `mix_hash`，不要在调用方另写一套哈希却声称兼容。

许可 MPL-2.0。本库是解码器，不是资源包。
