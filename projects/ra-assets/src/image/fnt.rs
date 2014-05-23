//! RA2/YR `fonT` 位图字体（如 `game.fnt`）。
//!
//! 布局：`fonT` 魔数 + 6×u32 头 + 65536×u16 码位点查表 + 字形槽位数据。
//! 每槽：1 字节宽度 + 1bpp 行数据（MSB 为左）。

use std::collections::HashMap;

use ra_types::{RaError, RaResult};

/// `fonT` 魔数（小端）。
pub const FONT_MAGIC: u32 = 0x546E_6F66;
/// 查表字节数。
const LOOKUP_TABLE_BYTES: usize = 65536 * 2;
/// 头字节数（魔数 + 6 字段）。
const HEADER_BYTES: usize = 4 + 6 * 4;

/// 已解析的位图字体。
#[derive(Debug, Clone)]
pub struct FntFile {
    /// 行高（含行距提示）。
    pub cell_height: u32,
    /// 字形位图行数。
    pub bitmap_rows: u32,
    /// 每行占用字节数。
    pub bytes_per_row: u32,
    /// 每字形槽字节数。
    pub glyph_stride: u32,
    glyphs: HashMap<u16, FntGlyph>,
}

/// 单个字形（白字透明底 RGBA）。
#[derive(Debug, Clone)]
pub struct FntGlyph {
    /// 像素宽。
    pub width: u32,
    /// `width × bitmap_rows × 4`。
    pub rgba: Vec<u8>,
}

impl FntFile {
    /// 从原始字节解析。
    pub fn parse(data: &[u8]) -> RaResult<Self> {
        let min_len = HEADER_BYTES + LOOKUP_TABLE_BYTES;
        if data.len() < min_len {
            return Err(RaError::Msg(format!("FNT 过短 · {} < {}", data.len(), min_len)));
        }
        let magic = u32::from_le_bytes(data[0..4].try_into().unwrap());
        if magic != FONT_MAGIC {
            return Err(RaError::Msg(format!("FNT 魔数错误 · 0x{magic:08X}")));
        }

        let field = |i: usize| -> u32 {
            let off = 4 + i * 4;
            u32::from_le_bytes(data[off..off + 4].try_into().unwrap())
        };
        let bytes_per_row = field(1);
        let bitmap_rows = field(2);
        let cell_height = field(3);
        let num_glyph_slots = field(4);
        let glyph_stride = field(5);
        if bytes_per_row == 0 || bitmap_rows == 0 || glyph_stride == 0 {
            return Err(RaError::Msg("FNT 头字段为零".into()));
        }

        let bitmap_data_size = (num_glyph_slots as usize).saturating_mul(glyph_stride as usize);
        let bitmap_data_offset = HEADER_BYTES + LOOKUP_TABLE_BYTES;
        if data.len() < bitmap_data_offset + bitmap_data_size {
            return Err(RaError::Msg(format!("FNT 字形数据不足 · {} < {}", data.len(), bitmap_data_offset + bitmap_data_size)));
        }

        let lookup_start = HEADER_BYTES;
        let bitmap_data = &data[bitmap_data_offset..bitmap_data_offset + bitmap_data_size];
        let mut glyphs = HashMap::new();
        for codepoint in 0u16..=u16::MAX {
            let lut_off = lookup_start + (codepoint as usize) * 2;
            let index = u16::from_le_bytes(data[lut_off..lut_off + 2].try_into().unwrap()) as u32;
            if index == 0 {
                continue;
            }
            let glyph_off = (glyph_stride as usize) * ((index - 1) as usize);
            let end = glyph_off + glyph_stride as usize;
            if end > bitmap_data.len() {
                continue;
            }
            let glyph_bytes = &bitmap_data[glyph_off..end];
            let width = u32::from(glyph_bytes[0]);
            if width == 0 {
                continue;
            }
            let rgba = decode_glyph_bitmap(&glyph_bytes[1..], width, bitmap_rows, bytes_per_row);
            glyphs.insert(codepoint, FntGlyph { width, rgba });
        }

        Ok(Self { cell_height, bitmap_rows, bytes_per_row, glyph_stride, glyphs })
    }

    /// 按 Unicode 码位点取字形。
    pub fn glyph(&self, codepoint: u16) -> Option<&FntGlyph> {
        self.glyphs.get(&codepoint)
    }

    /// 已加载字形数量。
    pub fn glyph_count(&self) -> usize {
        self.glyphs.len()
    }

    /// 文本像素宽（字形宽之和 + 字距 1px）。
    pub fn text_width(&self, text: &str) -> u32 {
        let mut width = 0u32;
        let mut count = 0u32;
        for ch in text.chars() {
            let cp = ch as u32;
            if cp > u32::from(u16::MAX) {
                continue;
            }
            if let Some(g) = self.glyphs.get(&(cp as u16)) {
                width = width.saturating_add(g.width);
                count += 1;
            }
        }
        if count > 1 {
            width = width.saturating_add(count - 1);
        }
        width
    }
}

fn decode_glyph_bitmap(bitmap: &[u8], width: u32, bitmap_rows: u32, bytes_per_row: u32) -> Vec<u8> {
    let mut rgba = vec![0u8; (width * bitmap_rows * 4) as usize];
    for row in 0..bitmap_rows {
        let row_start = (row * bytes_per_row) as usize;
        for px in 0..width {
            let byte_idx = row_start + (px / 8) as usize;
            let bit_idx = 7 - (px % 8);
            if byte_idx < bitmap.len() && ((bitmap[byte_idx] >> bit_idx) & 1) != 0 {
                let out = ((row * width + px) * 4) as usize;
                rgba[out] = 255;
                rgba[out + 1] = 255;
                rgba[out + 2] = 255;
                rgba[out + 3] = 255;
            }
        }
    }
    rgba
}
