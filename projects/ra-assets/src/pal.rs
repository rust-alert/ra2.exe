//! PAL 调色板：256 色 × RGB（VGA 6-bit，左移 2 位还原）。

use ra_types::{RaError, RaResult};

const COLOR_COUNT: usize = 256;
const PAL_FILE_SIZE: usize = COLOR_COUNT * 3;

/// RGBA 颜色。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Rgba {
    pub r: u8,
    pub g: u8,
    pub b: u8,
    pub a: u8,
}

impl Rgba {
    pub const fn rgb(r: u8, g: u8, b: u8) -> Self {
        Self { r, g, b, a: 255 }
    }

    pub const fn transparent() -> Self {
        Self {
            r: 0,
            g: 0,
            b: 0,
            a: 0,
        }
    }
}

/// 256 色调色板。索引 0 视为透明。
#[derive(Debug, Clone)]
pub struct Palette {
    pub colors: [Rgba; COLOR_COUNT],
}

impl Palette {
    /// 解析恰好 768 字节的 `.pal`。
    pub fn parse(data: &[u8]) -> RaResult<Self> {
        if data.len() != PAL_FILE_SIZE {
            return Err(RaError::Parse(format!(
                "pal 大小应为 {PAL_FILE_SIZE}，实际 {}",
                data.len()
            )));
        }

        let mut colors = [Rgba::rgb(0, 0, 0); COLOR_COUNT];
        for (i, color) in colors.iter_mut().enumerate() {
            let base = i * 3;
            let raw = [data[base], data[base + 1], data[base + 2]];
            let r = raw[0] << 2;
            let g = raw[1] << 2;
            let b = raw[2] << 2;
            // 索引 0，以及原生品红色键，按透明处理。
            let transparent = i == 0 || raw == [63, 0, 63];
            *color = Rgba {
                r,
                g,
                b,
                a: if transparent { 0 } else { 255 },
            };
        }
        Ok(Self { colors })
    }

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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_wrong_size() {
        assert!(Palette::parse(&[0u8; 10]).is_err());
    }

    #[test]
    fn shifts_vga_components() {
        let mut data = [0u8; PAL_FILE_SIZE];
        data[3] = 63;
        data[4] = 31;
        data[5] = 1;
        let pal = Palette::parse(&data).unwrap();
        assert_eq!(pal.colors[0].a, 0);
        assert_eq!(pal.colors[1], Rgba::rgb(252, 124, 4));
    }

    #[test]
    fn magenta_key_is_transparent() {
        let mut data = [0u8; PAL_FILE_SIZE];
        data[3..6].copy_from_slice(&[63, 0, 63]);
        let pal = Palette::parse(&data).unwrap();
        assert_eq!(pal.colors[1].a, 0);
        assert_eq!(pal.colors[1].r, 252);
        assert_eq!(pal.colors[1].b, 252);
    }
}
