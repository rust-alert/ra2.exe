//! PAL 调色板：256 色 × RGB（VGA 6-bit → 8-bit）。
//!
//! 扩色模式见 [`VgaExpandMode`]：默认满幅 `full`，也可用 `shift2`。
//! 进程默认可由桌面配置在启动时 [`set_default_vga_expand`] 覆盖。

use std::sync::atomic::{AtomicU8, Ordering};

use ra_types::{RaError, RaResult, VgaExpandMode};

const COLOR_COUNT: usize = 256;
const PAL_FILE_SIZE: usize = COLOR_COUNT * 3;

/// 0 = [`VgaExpandMode::Full`]，1 = [`VgaExpandMode::Shift2`]。
/// 默认 `Shift2`：与零售引擎 / 侧栏 chrome 常见路径一致（`63 → 252`）。
static DEFAULT_VGA_EXPAND: AtomicU8 = AtomicU8::new(1);

fn mode_to_tag(mode: VgaExpandMode) -> u8 {
    match mode {
        VgaExpandMode::Full => 0,
        VgaExpandMode::Shift2 => 1,
    }
}

fn tag_to_mode(tag: u8) -> VgaExpandMode {
    match tag {
        1 => VgaExpandMode::Shift2,
        _ => VgaExpandMode::Full,
    }
}

/// 设置后续 [`Palette::parse`] 使用的默认扩色模式（启动时由桌面配置写入）。
pub fn set_default_vga_expand(mode: VgaExpandMode) {
    DEFAULT_VGA_EXPAND.store(mode_to_tag(mode), Ordering::Relaxed);
}

/// 当前默认扩色模式。
pub fn default_vga_expand() -> VgaExpandMode {
    tag_to_mode(DEFAULT_VGA_EXPAND.load(Ordering::Relaxed))
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
    /// 解析恰好 768 字节的 `.pal`（使用进程默认扩色模式）。
    pub fn parse(data: &[u8]) -> RaResult<Self> {
        Self::parse_with(data, default_vga_expand())
    }

    /// 按指定扩色模式解析恰好 768 字节的 `.pal`。
    pub fn parse_with(data: &[u8], mode: VgaExpandMode) -> RaResult<Self> {
        if data.len() != PAL_FILE_SIZE {
            return Err(RaError::Parse(format!("pal 大小应为 {PAL_FILE_SIZE}，实际 {}", data.len())));
        }

        let mut colors = [Rgba::rgb(0, 0, 0); COLOR_COUNT];
        for (i, color) in colors.iter_mut().enumerate() {
            let base = i * 3;
            let raw = [data[base], data[base + 1], data[base + 2]];
            let r = mode.expand(raw[0]);
            let g = mode.expand(raw[1]);
            let b = mode.expand(raw[2]);
            // 索引 0，以及原生品红色键，按透明处理。
            let transparent = i == 0 || raw == [63, 0, 63];
            *color = Rgba { r, g, b, a: if transparent { 0 } else { 255 } };
        }
        Ok(Self { colors })
    }

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
