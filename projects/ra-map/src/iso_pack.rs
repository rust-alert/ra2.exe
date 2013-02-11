//! IsoMapPack5：base64 → LZO 分块 → 11 字节地形单元。

use ra_assets::IniDocument;
use ra_types::{RaError, RaResult};

use crate::base64;
use crate::lzo;

const CELL_RECORD_SIZE: usize = 11;

/// 解压后的一个等距地形单元。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct IsoCell {
    pub x: i16,
    pub y: i16,
    pub tile_num: i32,
    pub sub_tile: u8,
    pub z: u8,
    pub flags: u8,
}

/// 从场景 INI 的 `[IsoMapPack5]` 解码全部单元（丢弃 x=y=0 填充）。
pub fn decode_iso_map_pack(doc: &IniDocument) -> RaResult<Vec<IsoCell>> {
    let b64 = doc
        .numbered_section_concat("IsoMapPack5")
        .ok_or_else(|| RaError::Parse("缺少 [IsoMapPack5]".into()))?;
    let compressed = base64::base64_decode(&b64).map_err(|e| RaError::Parse(e))?;
    let raw = lzo::decompress_chunks(&compressed).map_err(|e| RaError::Parse(e.to_string()))?;
    Ok(parse_iso_cells(&raw))
}

fn parse_iso_cells(raw: &[u8]) -> Vec<IsoCell> {
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
        cells.push(IsoCell {
            x,
            y,
            tile_num,
            sub_tile,
            z,
            flags,
        });
    }
    cells
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_one_cell_record() {
        let mut raw = Vec::new();
        raw.extend_from_slice(&3i16.to_le_bytes());
        raw.extend_from_slice(&4i16.to_le_bytes());
        raw.extend_from_slice(&7i32.to_le_bytes());
        raw.push(1);
        raw.push(2);
        raw.push(0);
        // padding empty
        raw.extend_from_slice(&[0u8; 11]);
        let cells = parse_iso_cells(&raw);
        assert_eq!(cells.len(), 1);
        assert_eq!(cells[0].x, 3);
        assert_eq!(cells[0].y, 4);
        assert_eq!(cells[0].tile_num, 7);
        assert_eq!(cells[0].sub_tile, 1);
        assert_eq!(cells[0].z, 2);
    }
}
