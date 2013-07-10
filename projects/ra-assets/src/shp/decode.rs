//! SHP(TS) 行式 RLE-Zero 解压。

use ra_types::{RaError, RaResult};

/// 解压 bit1 置位的帧，返回恰好 `width * height` 个调色板索引。
pub fn decode_rle_frame(data: &[u8], width: usize, height: usize) -> RaResult<Vec<u8>> {
    let pixel_count = width.checked_mul(height).ok_or_else(|| RaError::Parse("shp RLE 尺寸溢出".into()))?;
    let mut pixels = Vec::with_capacity(pixel_count);
    let mut offset = 0usize;

    for row in 0..height {
        let row_start = offset;
        if row_start + 2 > data.len() {
            return Err(RaError::Parse(format!("shp RLE 行 {row} 前缀截断")));
        }
        let raw_length = u16::from_le_bytes([data[row_start], data[row_start + 1]]) as usize;
        if raw_length < 2 {
            return Err(RaError::Parse(format!("shp RLE 行 {row} 长度过小")));
        }
        let line_end = row_start.checked_add(raw_length).ok_or_else(|| RaError::Parse(format!("shp RLE 行 {row} 终点溢出")))?;
        if line_end > data.len() {
            return Err(RaError::Parse(format!("shp RLE 行 {row} 越界")));
        }

        offset = row_start + 2;
        let mut row_pixels = 0usize;
        while row_pixels < width {
            if offset >= line_end {
                return Err(RaError::Parse(format!("shp RLE 行 {row} 像素不足（{row_pixels}/{width}）")));
            }
            let byte = data[offset];
            offset += 1;
            if byte != 0 {
                pixels.push(byte);
                row_pixels += 1;
                continue;
            }
            if offset >= line_end {
                return Err(RaError::Parse(format!("shp RLE 行 {row} 零行程截断")));
            }
            let count = data[offset] as usize;
            offset += 1;
            let visible = count.min(width - row_pixels);
            pixels.extend(std::iter::repeat_n(0u8, visible));
            row_pixels += visible;
        }
        offset = line_end;
    }

    Ok(pixels)
}
