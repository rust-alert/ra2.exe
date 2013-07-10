# ra-map

本 crate 管的是 **一张图从 INI 变成单元格向量**的管道，以及剧院枚举到地形 MIX / 调色板文件名的映射。依赖 `ra-types` +
`ra-assets`（只要 INI 拼接能力）。没有 wgpu，没有 tick。

```
src/lib.rs        MapInfo::parse_ini / empty
src/theater.rs    Theater 五态 + mix/pal 名
src/iso_pack.rs   IsoMapPack5 → Vec<IsoCell>
src/base64.rs     标准 base64（跳过空白）
src/lzo.rs        自带完整 LZO1X + 分块包装
```

---

## 剧院表（`theater.rs`）——先背这张

| `Theater` | `as_str`  | INI 别名       | 挂载 MIX                  | 调色板       |
|-----------|-----------|----------------|---------------------------|--------------|
| Temperate | temperate | TEMPERATE, TEM | `temperat.mix`, `tem.mix` | `isotem.pal` |
| Snow      | snow      | SNOW, SNO      | `snow.mix`, `sno.mix`     | `isosno.pal` |
| Urban     | urban     | URBAN, URB     | `urban.mix`, `urb.mix`    | `isourb.pal` |
| Lunar     | lunar     | LUNAR, LUN     | `lunar.mix`, `lun.mix`    | `isolun.pal` |
| Desert    | desert    | DESERT, DES    | `desert.mix`, `des.mix`   | `isodes.pal` |

未知剧院字符串 → `RaError::Parse("未知剧院 `…`")`。桌面在 `MapInfo` 解析成功后，对 `theater_mix_names(map.theater)` 逐个
`mount_nested`。

---

## `MapInfo`（`lib.rs`）

字段：`edition`、`name`、`width`、`height`、`theater`、`cells: Vec<IsoCell>`。

`parse_ini(edition, name, bytes)`：

1. `IniDocument::parse`
2. 读 `[Map]` 的 `Size`——格式 `x,y,width,height`，取 **第 3、第 4 段** 当宽高；缺了就报错
3. `Theater` 键缺省当 `"TEMPERATE"`
4. 尝试解 IsoMapPack； **失败则 `cells` 置空向量继续返回 Ok**（吞掉解码错误，地图元数据仍可用）

`empty(edition, name)`：剧院默认 Temperate，尺寸 0，无单元格。桌面找不到启动地图时会落到这个占位。

测试 `parse_basic_map_ini`：断言 50×40、Snow、cells 空。

---

## IsoMapPack5 管线（`iso_pack.rs`）

`CELL_RECORD_SIZE = 11`。

```text
IniDocument::numbered_section_concat("IsoMapPack5")
    → base64_decode
    → lzo::decompress_chunks
    → 每 11 字节一格 IsoCell { x:i16, y:i16, tile_num:i32, sub_tile:u8, z:u8, flags:u8 }
    → 丢弃 x==0 && y==0 的填充记录
```

测试 `parse_one_cell_record` 覆盖「一格有效 + 填充被扔」。

---

## 为什么自己实现 LZO（`lzo.rs`）

没有引入外部 lzo crate。本文件实现 `lzo1x_decompress`（含 M1/M1'/M2/M3/M4 与变长编码路径）以及错误枚举：

```text
LzoError::InputTruncated
LzoError::OutputOverflow
LzoError::InvalidBackRef { distance, output_pos }
```

`decompress_chunks` 包装格式：重复 `[u16 src_len][u16 dst_len][bytes…]`，直到 `src_len==dst_len==0` 结束。地图 IsoMapPack
走的是分块包装，不是裸 LZO 流。

测试：`test_literal_only_stream`（Hello + EOS `0x11 0x00 0x00`）、`test_chunk_wrapper`、`test_empty_input`。

---

## base64（`base64.rs`）

标准字母表；空白可跳过；padding 之后不允许再跟数据。错误字符串目前是英文（与仓库多数中文错误不完全一致）——改文案时注意测试是否断言了原文。

测试覆盖空、hello、无 padding、双 padding、空白、非法字符、二进制往返。

---

## 和桌面启动的衔接

桌面候选地图名硬编码为：`mp01t4.map`、`mp01t2.map`、`mp02t4.map`。谁先在 VFS 里读到并 `parse_ini` 成功，谁就定剧院并挂剧院
MIX。本 crate 不决定候选列表。

## 构建

```shell
cargo test -p ra-map
```

许可 MPL-2.0。地图与剧院资源由用户自备。若你要画地形，下一步数据在 `cells` 与 `ra-assets` 的 TMP/SHP，不在本 crate 里找 GPU
代码。
