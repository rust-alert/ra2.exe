//! 房屋色 remapping：调色板索引 16..=31（Westwood 惯例）。

use crate::image::pal::{Palette, Rgba};

/// 可替换色带长度。
pub const HOUSE_REMAP_COUNT: usize = 16;
/// 色带起始索引。
pub const HOUSE_REMAP_FIRST: usize = 16;

/// rules `[Colors]` 用的 HSV（三分量均为 0..=255）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Hsv {
    /// 色相。
    pub h: u8,
    /// 饱和度。
    pub s: u8,
    /// 明度。
    pub v: u8,
}

/// 把 0..=255 HSV 转成 RGB（常见字节域算法）。
pub fn hsv_to_rgb(hsv: Hsv) -> Rgba {
    if hsv.s == 0 {
        return Rgba::rgb(hsv.v, hsv.v, hsv.v);
    }
    let region = u16::from(hsv.h) / 43;
    let remainder = (u16::from(hsv.h) - region * 43) * 6;
    let p = (u16::from(hsv.v) * (255 - u16::from(hsv.s))) / 255;
    let q = (u16::from(hsv.v) * (255 - (u16::from(hsv.s) * remainder) / 255)) / 255;
    let t = (u16::from(hsv.v) * (255 - (u16::from(hsv.s) * (255 - remainder)) / 255)) / 255;
    let (r, g, b) = match region {
        0 => (u16::from(hsv.v), t, p),
        1 => (q, u16::from(hsv.v), p),
        2 => (p, u16::from(hsv.v), t),
        3 => (p, q, u16::from(hsv.v)),
        4 => (t, p, u16::from(hsv.v)),
        _ => (u16::from(hsv.v), p, q),
    };
    Rgba::rgb(r as u8, g as u8, b as u8)
}

/// 根据主色生成 16 档亮度斜坡（0=最亮 … 15=最暗）。
pub fn build_remap_ramp(primary: Rgba) -> [Rgba; HOUSE_REMAP_COUNT] {
    let mut ramp = [Rgba::rgb(0, 0, 0); HOUSE_REMAP_COUNT];
    for (i, slot) in ramp.iter_mut().enumerate() {
        let t = 1.0 - (i as f32) / ((HOUSE_REMAP_COUNT - 1) as f32);
        let f = 0.20 + 0.75 * t;
        *slot = Rgba { r: scale_channel(primary.r, f), g: scale_channel(primary.g, f), b: scale_channel(primary.b, f), a: 255 };
    }
    ramp
}

/// 按 HSV 生成色带：固定 H/S，V 从方案值降到约 18%。
pub fn build_hsv_remap_ramp(hsv: Hsv) -> [Rgba; HOUSE_REMAP_COUNT] {
    let mut ramp = [Rgba::rgb(0, 0, 0); HOUSE_REMAP_COUNT];
    for (i, slot) in ramp.iter_mut().enumerate() {
        let t = (i as f32) / ((HOUSE_REMAP_COUNT - 1) as f32);
        let value = (f32::from(hsv.v) * (1.0 - 0.82 * t)).round().clamp(0.0, 255.0) as u8;
        *slot = hsv_to_rgb(Hsv { h: hsv.h, s: hsv.s, v: value });
    }
    ramp
}

fn scale_channel(ch: u8, f: f32) -> u8 {
    (f32::from(ch) * f).round().clamp(0.0, 255.0) as u8
}

/// 常见阵营主色；中立等返回 `None` 表示不 remap。
pub fn owner_primary_color(owner: &str) -> Option<Rgba> {
    match owner.to_ascii_uppercase().as_str() {
        "NEUTRAL" | "SPECIAL" | "CIVILIAN" => None,
        "AMERICANS" | "ALLIED" | "ALLIANCE" | "GDI" => Some(Rgba::rgb(0, 80, 220)),
        "BRITISH" | "ENGLISH" => Some(Rgba::rgb(200, 40, 40)),
        "FRENCH" => Some(Rgba::rgb(0, 180, 220)),
        "GERMANS" | "GERMAN" => Some(Rgba::rgb(120, 120, 120)),
        "AFRICANS" | "KOREANS" | "KOREAN" => Some(Rgba::rgb(220, 140, 20)),
        "RUSSIANS" | "SOVIET" | "NOD" => Some(Rgba::rgb(180, 20, 20)),
        "CONFEDERATION" | "CUBA" | "CUBAN" => Some(Rgba::rgb(20, 140, 40)),
        "ARABS" | "LIBYA" | "LIBYAN" | "IRAQ" | "IRAQI" => Some(Rgba::rgb(140, 140, 40)),
        "YURI" | "YURICOUNTRY" => Some(Rgba::rgb(160, 40, 200)),
        _ => Some(Rgba::rgb(180, 180, 40)),
    }
}

fn apply_ramp(pal: &Palette, ramp: &[Rgba; HOUSE_REMAP_COUNT]) -> Palette {
    let mut out = pal.clone();
    for (i, c) in ramp.iter().enumerate() {
        out.colors[HOUSE_REMAP_FIRST + i] = *c;
    }
    out
}

impl Palette {
    /// 用房屋色带替换索引 16..=31；其余不变。
    pub fn with_house_remap(&self, primary: Rgba) -> Self {
        apply_ramp(self, &build_remap_ramp(primary))
    }

    /// 用 rules `[Colors]` HSV 生成色带。
    pub fn with_hsv_remap(&self, hsv: Hsv) -> Self {
        apply_ramp(self, &build_hsv_remap_ramp(hsv))
    }

    /// 按 `owner` 名尝试 remap；中立等返回自身克隆。
    pub fn for_owner(&self, owner: &str) -> Self {
        match owner_primary_color(owner) {
            Some(c) => self.with_house_remap(c),
            None => self.clone(),
        }
    }
}
