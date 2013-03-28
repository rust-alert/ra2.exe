//! 房屋色 remapping：调色板索引 16..=31（Westwood 惯例）。

use crate::pal::{Palette, Rgba};

/// 可替换色带长度。
pub const HOUSE_REMAP_COUNT: usize = 16;
/// 色带起始索引。
pub const HOUSE_REMAP_FIRST: usize = 16;

/// 根据主色生成 16 档亮度斜坡（0=最亮 … 15=最暗）。
pub fn build_remap_ramp(primary: Rgba) -> [Rgba; HOUSE_REMAP_COUNT] {
    let mut ramp = [Rgba::rgb(0, 0, 0); HOUSE_REMAP_COUNT];
    for (i, slot) in ramp.iter_mut().enumerate() {
        // 0.95 → 0.20，避免全白/全黑。
        let t = 1.0 - (i as f32) / ((HOUSE_REMAP_COUNT - 1) as f32);
        let f = 0.20 + 0.75 * t;
        *slot = Rgba {
            r: scale_channel(primary.r, f),
            g: scale_channel(primary.g, f),
            b: scale_channel(primary.b, f),
            a: 255,
        };
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
        "AMERICANS" | "ALLIED" => Some(Rgba::rgb(0, 80, 220)),
        "BRITISH" | "ENGLISH" => Some(Rgba::rgb(200, 40, 40)),
        "FRENCH" => Some(Rgba::rgb(0, 180, 220)),
        "GERMANS" | "GERMAN" => Some(Rgba::rgb(120, 120, 120)),
        "KOREANS" | "KOREAN" => Some(Rgba::rgb(220, 140, 20)),
        "RUSSIANS" | "SOVIET" => Some(Rgba::rgb(180, 20, 20)),
        "CUBA" | "CUBAN" => Some(Rgba::rgb(20, 140, 40)),
        "LIBYA" | "LIBYAN" => Some(Rgba::rgb(40, 160, 40)),
        "IRAQ" | "IRAQI" => Some(Rgba::rgb(140, 140, 40)),
        "YURI" | "YURICOUNTRY" => Some(Rgba::rgb(160, 40, 200)),
        _ => Some(Rgba::rgb(180, 180, 40)),
    }
}

impl Palette {
    /// 用房屋色带替换索引 16..=31；其余不变。
    pub fn with_house_remap(&self, primary: Rgba) -> Self {
        let ramp = build_remap_ramp(primary);
        let mut out = self.clone();
        for (i, c) in ramp.iter().enumerate() {
            out.colors[HOUSE_REMAP_FIRST + i] = *c;
        }
        out
    }

    /// 按 `owner` 名尝试 remap；中立等返回自身克隆。
    pub fn for_owner(&self, owner: &str) -> Self {
        match owner_primary_color(owner) {
            Some(c) => self.with_house_remap(c),
            None => self.clone(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn neutral_skips_remap() {
        assert!(owner_primary_color("Neutral").is_none());
        assert!(owner_primary_color("Americans").is_some());
    }

    #[test]
    fn ramp_replaces_band_only() {
        let mut colors = [Rgba::rgb(1, 2, 3); 256];
        colors[0] = Rgba::transparent();
        colors[40] = Rgba::rgb(9, 9, 9);
        let pal = Palette { colors };
        let remapped = pal.with_house_remap(Rgba::rgb(255, 0, 0));
        assert_eq!(remapped.colors[40], Rgba::rgb(9, 9, 9));
        assert_eq!(remapped.colors[16].a, 255);
        assert!(remapped.colors[16].r > remapped.colors[31].r);
        assert_eq!(remapped.colors[16].g, 0);
    }
}
