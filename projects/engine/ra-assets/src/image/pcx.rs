//! PCX（ZSoft）8-bit 索引图解析 → RGBA。
//!
//! 零售 `title.pcx` / `load.pcx` 走此路径；不依赖外部 image crate。

use ra_types::{RaError, RaResult};

/// 解析后的 PCX 光栅（不透明 RGBA）。
#[derive(Debug, Clone)]
pub struct PcxImage {
    /// 像素宽。
    pub width: u32,
    /// 像素高。
    pub height: u32,
    /// 行主序 RGBA（每像素 4 字节）。
    pub rgba: Vec<u8>,
}

const HEADER_SIZE: usize = 128;
const VGA_PALETTE_MARKER: u8 = 0x0C;
const VGA_PALETTE_BYTES: usize = 1 + 256 * 3;

/// 解析常见 Westwood 8-bit RLE PCX（含末尾 VGA 调色板）。
pub fn parse_pcx(data: &[u8]) -> RaResult<PcxImage> {
    if data.len() < HEADER_SIZE + VGA_PALETTE_BYTES {
        return Err(RaError::Parse(format!("pcx 过短：{} 字节", data.len())));
    }
    if data[0] != 0x0A {
        return Err(RaError::Parse(format!("pcx 魔数非法：0x{:02X}", data[0])));
    }
    let encoding = data[2];
    let bits_per_pixel = data[3];
    let xmin = u16::from_le_bytes([data[4], data[5]]) as u32;
    let ymin = u16::from_le_bytes([data[6], data[7]]) as u32;
    let xmax = u16::from_le_bytes([data[8], data[9]]) as u32;
    let ymax = u16::from_le_bytes([data[10], data[11]]) as u32;
    if xmax < xmin || ymax < ymin {
        return Err(RaError::Parse(format!("pcx 尺寸非法：{xmin}..{xmax} × {ymin}..{ymax}")));
    }
    let width = xmax - xmin + 1;
    let height = ymax - ymin + 1;
    let n_planes = data[65];
    let bytes_per_line = u16::from_le_bytes([data[66], data[67]]) as usize;
    if encoding != 1 {
        return Err(RaError::Parse(format!("pcx 仅支持 RLE encoding=1，实际 {encoding}")));
    }
    if bits_per_pixel != 8 || n_planes != 1 {
        return Err(RaError::Parse(format!("pcx 仅支持 8-bit 单平面，实际 bpp={bits_per_pixel} planes={n_planes}")));
    }
    if bytes_per_line < width as usize {
        return Err(RaError::Parse(format!("pcx bytes_per_line={bytes_per_line} < width={width}")));
    }

    let pal_off = data.len() - VGA_PALETTE_BYTES;
    if data[pal_off] != VGA_PALETTE_MARKER {
        return Err(RaError::Parse(format!("pcx 末尾无 VGA 调色板标记 0x0C，实际 0x{:02X}", data[pal_off])));
    }
    let mut palette = [[0u8; 3]; 256];
    for i in 0..256 {
        let o = pal_off + 1 + i * 3;
        palette[i] = [data[o], data[o + 1], data[o + 2]];
    }

    let compressed = &data[HEADER_SIZE..pal_off];
    let mut indices = vec![0u8; bytes_per_line * height as usize];
    let mut di = 0usize;
    let mut si = 0usize;
    while di < indices.len() {
        if si >= compressed.len() {
            return Err(RaError::Parse(format!("pcx RLE 耗尽：已写 {di}/{}，压缩区 {}", indices.len(), compressed.len())));
        }
        let b = compressed[si];
        si += 1;
        if b & 0xC0 == 0xC0 {
            let run = (b & 0x3F) as usize;
            if si >= compressed.len() {
                return Err(RaError::Parse("pcx RLE 缺值字节".into()));
            }
            let v = compressed[si];
            si += 1;
            let end = (di + run).min(indices.len());
            indices[di..end].fill(v);
            di = end;
        }
        else {
            indices[di] = b;
            di += 1;
        }
    }

    let mut rgba = vec![0u8; (width * height * 4) as usize];
    for y in 0..height as usize {
        let row = y * bytes_per_line;
        for x in 0..width as usize {
            let idx = indices[row + x] as usize;
            let [r, g, b] = palette[idx];
            let o = (y * width as usize + x) * 4;
            rgba[o] = r;
            rgba[o + 1] = g;
            rgba[o + 2] = b;
            rgba[o + 3] = 255;
        }
    }

    Ok(PcxImage { width, height, rgba })
}
