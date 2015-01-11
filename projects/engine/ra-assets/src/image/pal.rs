//! PAL 调色板：256 色 × RGB（VGA 6-bit → 8-bit）。
//!
//! 扩色用 `(v & 63) * 255 / 63`（满幅到 255），对齐常见 MIX 工具预览；
//! 不是 `v << 2`（最高只到 252）。

use ra_types::{RaError, RaResult};

const COLOR_COUNT: usize = 256;
const PAL_FILE_SIZE: usize = COLOR_COUNT * 3;

/// VGA 6-bit 分量扩到 8-bit（`0..=63` → `0..=255`）。
#[inline]
fn expand_vga6(v: u8) -> u8 {
    ((u16::from(v & 63) * 255) / 63) as u8
}

/// RGBA 颜色。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Rgba {
    /// 红。
    pub r: u8,
    /// 绿。
    pub g: u8,
    /// 蓝。
    pub b: u8,
    /// 透明度（0=全透明）。
    pub a: u8,
}

impl Rgba {
    /// 不透明 RGB。
    pub const fn rgb(r: u8, g: u8, b: u8) -> Self {
        Self { r, g, b, a: 255 }
    }

    /// 全透明黑。
    pub const fn transparent() -> Self {
        Self { r: 0, g: 0, b: 0, a: 0 }
    }
}

/// 256 色调色板。索引 0 视为透明。
#[derive(Debug, Clone)]
pub struct Palette {
    /// 256 个 RGBA 槽。
    pub colors: [Rgba; COLOR_COUNT],
}

impl Palette {
    /// 解析恰好 768 字节的 `.pal`。
    pub fn parse(data: &[u8]) -> RaResult<Self> {
        if data.len() != PAL_FILE_SIZE {
            return Err(RaError::Parse(format!("pal 大小应为 {PAL_FILE_SIZE}，实际 {}", data.len())));
        }

        let mut colors = [Rgba::rgb(0, 0, 0); COLOR_COUNT];
        for (i, color) in colors.iter_mut().enumerate() {
            let base = i * 3;
            let raw = [data[base], data[base + 1], data[base + 2]];
            let r = expand_vga6(raw[0]);
            let g = expand_vga6(raw[1]);
            let b = expand_vga6(raw[2]);
            // 索引 0，以及原生品红色键，按透明处理。
            let transparent = i == 0 || raw == [63, 0, 63];
            *color = Rgba { r, g, b, a: if transparent { 0 } else { 255 } };
        }
        Ok(Self { colors })
    }

    /// 展开为 256×RGBA 字节数组（供 GPU / 预览上传）。
    /// 展开为 256×RGBA 字节（共 1024）。
    pub fn to_rgba_bytes(&self) -> [u8; COLOR_COUNT * 4] {
        let mut out = [0u8; COLOR_COUNT * 4];
        for (i, c) in self.colors.iter().enumerate() {
            let o = i * 4;
            out[o] = c.r;
            out[o + 1] = c.g;
            out[o + 2] = c.b;
            out[o + 3] = c.a;
        }
        out
    }
}
