//! SHP(TS) 精灵：帧头 + 原始/RLE-Zero 像素。

mod decode;

use ra_types::{RaError, RaResult};

use crate::Palette;

pub use decode::decode_rle_frame;

const FORMAT_RLE_ZERO_BIT: u8 = 0x02;

/// 解析后的 SHP 文件。
#[derive(Debug, Clone)]
pub struct ShpFile {
    /// 画布宽。
    pub width: u16,
    /// 画布高。
    pub height: u16,
    /// 各帧。
    pub frames: Vec<ShpFrame>,
}

/// 单帧：调色板索引像素。
#[derive(Debug, Clone)]
pub struct ShpFrame {
    /// 相对画布原点的 X。
    pub frame_x: u16,
    /// 相对画布原点的 Y。
    pub frame_y: u16,
    /// 帧宽。
    pub frame_width: u16,
    /// 帧高。
    pub frame_height: u16,
    /// 格式标志（bit1 = RLE-Zero）。
    pub format: u8,
    /// 行优先调色板索引。
    pub pixels: Vec<u8>,
}

impl ShpFile {
    /// 解析 SHP(TS) 字节。
    pub fn parse(data: &[u8]) -> RaResult<Self> {
        if data.len() < 8 {
            return Err(RaError::Parse("shp 头过小".into()));
        }
        let zero = read_u16(data, 0);
        if zero != 0 {
            return Err(RaError::Parse(format!("非 SHP(TS) 标记: 首字 {zero}")));
        }
        let width = read_u16(data, 2);
        let height = read_u16(data, 4);
        let frame_count = read_u16(data, 6) as usize;
        let headers_end = 8 + frame_count * 24;
        if data.len() < headers_end {
            return Err(RaError::Parse("shp 帧头截断".into()));
        }

        let mut frames = Vec::with_capacity(frame_count);
        for i in 0..frame_count {
            let o = 8 + i * 24;
            let frame_x = read_u16(data, o);
            let frame_y = read_u16(data, o + 2);
            let frame_width = read_u16(data, o + 4);
            let frame_height = read_u16(data, o + 6);
            let format = data[o + 8];
            let data_offset = read_u32(data, o + 20) as usize;

            if frame_width == 0 || frame_height == 0 {
                frames.push(ShpFrame { frame_x, frame_y, frame_width, frame_height, format, pixels: Vec::new() });
                continue;
            }

            let pixel_count = frame_width as usize * frame_height as usize;
            if data_offset == 0 || data_offset >= data.len() {
                return Err(RaError::Parse(format!("shp 帧 {i} 数据偏移无效")));
            }
            let slice = &data[data_offset..];
            let pixels = if (format & FORMAT_RLE_ZERO_BIT) != 0 {
                decode_rle_frame(slice, frame_width as usize, frame_height as usize)?
            }
            else {
                if slice.len() < pixel_count {
                    return Err(RaError::Parse(format!("shp 帧 {i} 原始像素截断")));
                }
                slice[..pixel_count].to_vec()
            };

            frames.push(ShpFrame { frame_x, frame_y, frame_width, frame_height, format, pixels });
        }

        Ok(Self { width, height, frames })
    }

    /// 帧数量。
    pub fn frame_count(&self) -> usize {
        self.frames.len()
    }
}

impl ShpFrame {
    /// 按调色板展开为 RGBA（行优先，尺寸为 frame_width × frame_height）。
    pub fn to_rgba(&self, palette: &Palette) -> Vec<u8> {
        let mut out = Vec::with_capacity(self.pixels.len() * 4);
        for &idx in &self.pixels {
            let c = palette.colors[idx as usize];
            out.extend_from_slice(&[c.r, c.g, c.b, c.a]);
        }
        out
    }
}

fn read_u16(data: &[u8], offset: usize) -> u16 {
    u16::from_le_bytes(data[offset..offset + 2].try_into().unwrap())
}

fn read_u32(data: &[u8], offset: usize) -> u32 {
    u32::from_le_bytes(data[offset..offset + 4].try_into().unwrap())
}
