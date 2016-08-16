//! `[OverlayPack]` / `[OverlayDataPack]`：base64 → LCW 分块 → 512×512 格网。

use ra_assets::{IniDocument, numbered_section_concat};
use ra_types::{RaError, RaResult};

use crate::{base64, lcw};

/// 覆盖层格网边长（固定 512）。
pub const OVERLAY_GRID: usize = 512;
/// 总格数。
pub const OVERLAY_CELLS: usize = OVERLAY_GRID * OVERLAY_GRID;
/// `0xFF` 表示该格无覆盖层。
pub const NO_OVERLAY: u8 = 0xFF;

/// 一格有效覆盖层。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct OverlayCell {
    /// 格子 X。
    pub x: u16,
    /// 格子 Y。
    pub y: u16,
    /// 覆盖层类型 id。
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
        out.push(OverlayCell { x, y, overlay_id, data: data_byte });
    }
    Ok(out)
}

fn decode_pack_section(doc: &IniDocument, section: &str) -> RaResult<Vec<u8>> {
    let b64 = numbered_section_concat(doc, section).ok_or_else(|| RaError::Parse(format!("缺少 [{section}]")))?;
    let compressed = base64::base64_decode(&b64).map_err(RaError::Parse)?;
    lcw::decompress_chunks(&compressed).map_err(|e| RaError::Parse(e.to_string()))
}
