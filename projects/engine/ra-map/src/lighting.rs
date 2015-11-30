//! 地图 `[Lighting]`：环境光与每格 RGB tint。
//!
//! 标量按原版 Scenario 量化：Ambient/RGB ×100（+0.01 截断），Ground/Level ×250；
//! 内部单位 `1000 == 1.0`，再与通道相乘得到叠画倍率。

use ra_assets::IniDocument;

/// 内部光强单位：`1000 == 1.0`。
const LIGHT_UNIT: i32 = 1000;
/// 标量上限（对应 tint 通道乘积上限约 2.0）。
const LIGHT_CLAMP_MAX: i32 = 2000;

/// 地图 `[Lighting]` 全局参数。
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct LightingConfig {
    /// 基础亮度（缺省 `1.0`）。
    pub ambient: f32,
    /// 红通道倍率（缺省 `1.0`）。
    pub red: f32,
    /// 绿通道倍率（缺省 `1.0`）。
    pub green: f32,
    /// 蓝通道倍率（缺省 `1.0`）。
    pub blue: f32,
    /// 地面压暗项（缺省 `0.20`；按 ×250 量化后从 ambient 单位中减去）。
    pub ground: f32,
    /// 每级海拔对 ambient 的增量（缺省 `0.032`）。
    pub level: f32,
}

impl Default for LightingConfig {
    fn default() -> Self {
        Self {
            ambient: 1.0,
            red: 1.0,
            green: 1.0,
            blue: 1.0,
            ground: 0.20,
            level: 0.032,
        }
    }
}

impl LightingConfig {
    /// 无压暗、通道为 1 的恒等配置（测试用；零售缺省仍含 `Ground=0.20`）。
    pub const fn identity() -> Self {
        Self {
            ambient: 1.0,
            red: 1.0,
            green: 1.0,
            blue: 1.0,
            ground: 0.0,
            level: 0.0,
        }
    }
}

/// 从地图 INI 解析 `[Lighting]`；缺节或缺键用零售缺省。
pub fn parse_lighting(doc: &IniDocument) -> LightingConfig {
    let mut cfg = LightingConfig::default();
    if doc.get("Lighting", "Ambient").is_none()
        && doc.get("Lighting", "Red").is_none()
        && doc.get("Lighting", "Ground").is_none()
        && doc.get("Lighting", "Level").is_none()
        && doc.get("Lighting", "Green").is_none()
        && doc.get("Lighting", "Blue").is_none()
    {
        // 整节缺失时仍返回缺省（含 Ground），与「有节但键全缺」一致。
        return cfg;
    }
    if let Some(v) = doc.get("Lighting", "Ambient").and_then(parse_f32) {
        cfg.ambient = v;
    }
    if let Some(v) = doc.get("Lighting", "Red").and_then(parse_f32) {
        cfg.red = v;
    }
    if let Some(v) = doc.get("Lighting", "Green").and_then(parse_f32) {
        cfg.green = v;
    }
    if let Some(v) = doc.get("Lighting", "Blue").and_then(parse_f32) {
        cfg.blue = v;
    }
    if let Some(v) = doc.get("Lighting", "Ground").and_then(parse_f32) {
        cfg.ground = v;
    }
    if let Some(v) = doc.get("Lighting", "Level").and_then(parse_f32) {
        cfg.level = v;
    }
    cfg
}

fn parse_f32(raw: &str) -> Option<f32> {
    raw.trim().parse::<f32>().ok()
}

fn quantize(value: f32, scale: i32) -> i32 {
    (f64::from(value) * f64::from(scale) + 0.01) as i32
}

/// 海拔 `z` 的光照标量（`1.0` = 满亮）。
pub fn cell_light_scalar(config: &LightingConfig, z: u8) -> f32 {
    let ambient = quantize(config.ambient, 100).wrapping_mul(10);
    let ground = quantize(config.ground, 250);
    let level = quantize(config.level, 250);
    let scalar = ambient
        .wrapping_add(level.wrapping_mul(i32::from(z)))
        .wrapping_sub(ground)
        .clamp(0, LIGHT_CLAMP_MAX);
    scalar as f32 / LIGHT_UNIT as f32
}

/// 海拔 `z` 的 RGB tint（乘到 8-bit 通道上）。
pub fn cell_tint(config: &LightingConfig, z: u8) -> [f32; 3] {
    let scalar = cell_light_scalar(config, z);
    let channel = |c: f32| quantize(c, 100).wrapping_mul(10) as f32 / LIGHT_UNIT as f32;
    [
        channel(config.red) * scalar,
        channel(config.green) * scalar,
        channel(config.blue) * scalar,
    ]
}

/// 地形 TMP 统一用地面海拔 tint，避免同砖因邻格 `z` 不同出现接缝。
pub fn terrain_tint(config: &LightingConfig) -> [f32; 3] {
    cell_tint(config, 0)
}

/// 就地乘 RGB tint；`alpha` 不变。
pub fn apply_rgba_tint(rgba: &mut [u8], tint: [f32; 3]) {
    for px in rgba.chunks_exact_mut(4) {
        px[0] = mul_channel(px[0], tint[0]);
        px[1] = mul_channel(px[1], tint[1]);
        px[2] = mul_channel(px[2], tint[2]);
    }
}

fn mul_channel(value: u8, tint: f32) -> u8 {
    (f32::from(value) * tint).clamp(0.0, 255.0) as u8
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_ground_level_tint_is_0_95() {
        let tint = terrain_tint(&LightingConfig::default());
        assert!((tint[0] - 0.95).abs() < 1e-3);
        assert!((tint[1] - 0.95).abs() < 1e-3);
        assert!((tint[2] - 0.95).abs() < 1e-3);
    }

    #[test]
    fn identity_config_is_full_bright() {
        let tint = terrain_tint(&LightingConfig::identity());
        assert!((tint[0] - 1.0).abs() < 1e-3);
    }

    #[test]
    fn level_raises_high_cells() {
        let cfg = LightingConfig::default();
        let low = cell_light_scalar(&cfg, 0);
        let high = cell_light_scalar(&cfg, 10);
        assert!(high > low);
    }
}
