//! IsoMapPack5：base64 → LZO 分块 → 11 字节地形单元。

use ra_assets::IniDocument;
use ra_types::{RaError, RaResult};

use crate::{base64, lzo};

const CELL_RECORD_SIZE: usize = 11;

/// 解压后的一个等距地形单元。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct IsoCell {
    /// 格子 X。
    pub x: i16,
    /// 格子 Y。
    pub y: i16,
    /// 全局砖块编号（剧院 tileset；`0xFFFF` 表示 Clear 哨兵，绘制侧会归一到 0）。
    pub tile_num: i32,
    /// TMP 内子砖索引。
    pub sub_tile: u8,
    /// 高度档。
    pub z: u8,
    /// 原版标志字节。
    pub flags: u8,
}

/// 从场景 INI 的 `[IsoMapPack5]` 解码全部单元（丢弃 x=y=0 填充）。
pub fn decode_iso_map_pack(doc: &IniDocument) -> RaResult<Vec<IsoCell>> {
    let b64 = doc.numbered_section_concat("IsoMapPack5").ok_or_else(|| RaError::Parse("缺少 [IsoMapPack5]".into()))?;
    let compressed = base64::base64_decode(&b64).map_err(|e| RaError::Parse(e))?;
    let raw = lzo::decompress_chunks(&compressed).map_err(|e| RaError::Parse(e.to_string()))?;
    Ok(parse_iso_cells(&raw))
}

/// 将解压后的 IsoMapPack 原始字节解析为单元表（跳过 x=y=0 填充）。
pub fn parse_iso_cells(raw: &[u8]) -> Vec<IsoCell> {
    let count = raw.len() / CELL_RECORD_SIZE;
    let mut cells = Vec::with_capacity(count);
    for i in 0..count {
        let o = i * CELL_RECORD_SIZE;
        let x = i16::from_le_bytes([raw[o], raw[o + 1]]);
        let y = i16::from_le_bytes([raw[o + 2], raw[o + 3]]);
        let tile_num = i32::from_le_bytes([raw[o + 4], raw[o + 5], raw[o + 6], raw[o + 7]]);
        let sub_tile = raw[o + 8];
        let z = raw[o + 9];
        let flags = raw[o + 10];
        if x == 0 && y == 0 {
            continue;
        }
        cells.push(IsoCell { x, y, tile_num, sub_tile, z, flags });
    }
    cells
}
