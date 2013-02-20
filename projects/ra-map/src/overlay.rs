//! `[OverlayPack]` / `[OverlayDataPack]`：base64 → LCW 分块 → 512×512 格网。

use ra_assets::IniDocument;
use ra_types::{RaError, RaResult};

use crate::base64;
use crate::lcw;

/// 覆盖层格网边长（固定 512）。
pub const OVERLAY_GRID: usize = 512;
/// 总格数。
pub const OVERLAY_CELLS: usize = OVERLAY_GRID * OVERLAY_GRID;
/// `0xFF` 表示该格无覆盖层。
pub const NO_OVERLAY: u8 = 0xFF;

/// 一格有效覆盖层。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct OverlayCell {
    pub x: u16,
    pub y: u16,
    pub overlay_id: u8,
    /// 来自 OverlayDataPack：矿密度 / 墙帧等。
    pub data: u8,
}

/// 从场景 INI 解码覆盖层条目（跳过 `0xFF` 空格）。
pub fn decode_overlay_packs(doc: &IniDocument) -> RaResult<Vec<OverlayCell>> {
    let identity = decode_pack_section(doc, "OverlayPack")?;
    let data = match decode_pack_section(doc, "OverlayDataPack") {
        Ok(bytes) => bytes,
        Err(_) => Vec::new(),
    };

    let limit = identity.len().min(OVERLAY_CELLS);
    let mut out = Vec::new();
    for idx in 0..limit {
        let overlay_id = identity[idx];
        if overlay_id == NO_OVERLAY {
            continue;
        }
        let x = (idx % OVERLAY_GRID) as u16;
        let y = (idx / OVERLAY_GRID) as u16;
        let data_byte = data.get(idx).copied().unwrap_or(0);
        out.push(OverlayCell {
            x,
            y,
            overlay_id,
            data: data_byte,
        });
    }
    Ok(out)
}

fn decode_pack_section(doc: &IniDocument, section: &str) -> RaResult<Vec<u8>> {
    let b64 = doc
        .numbered_section_concat(section)
        .ok_or_else(|| RaError::Parse(format!("缺少 [{section}]")))?;
    let compressed = base64::base64_decode(&b64).map_err(RaError::Parse)?;
    lcw::decompress_chunks(&compressed).map_err(|e| RaError::Parse(e.to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn pack_section(name: &str, grid: &[u8]) -> String {
        // 单块：字面量 + 结束，再包分块帧。
        assert!(grid.len() <= 63);
        let mut lcw = Vec::new();
        lcw.push(0x80 | (grid.len() as u8));
        lcw.extend_from_slice(grid);
        lcw.push(0x80);
        let mut framed = Vec::new();
        framed.extend_from_slice(&(lcw.len() as u16).to_le_bytes());
        framed.extend_from_slice(&(grid.len() as u16).to_le_bytes());
        framed.extend_from_slice(&lcw);
        let b64 = base64::base64_encode(&framed);
        format!("[{name}]\n1={b64}\n")
    }

    #[test]
    fn decode_two_cells() {
        // 仅前 4 字节有意义：id, FF, id, FF → 两格有效。
        let grid = [10u8, NO_OVERLAY, 20, NO_OVERLAY];
        let data = [3u8, 0, 7, 0];
        let text = format!(
            "{}{}",
            pack_section("OverlayPack", &grid),
            pack_section("OverlayDataPack", &data)
        );
        let doc = IniDocument::parse(text.as_bytes()).unwrap();
        let cells = decode_overlay_packs(&doc).unwrap();
        assert_eq!(cells.len(), 2);
        assert_eq!(
            cells[0],
            OverlayCell {
                x: 0,
                y: 0,
                overlay_id: 10,
                data: 3
            }
        );
        assert_eq!(
            cells[1],
            OverlayCell {
                x: 2,
                y: 0,
                overlay_id: 20,
                data: 7
            }
        );
    }
}
