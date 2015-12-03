//! 地图 `[Lighting]`：环境光、点光源与每格 RGB tint。
//!
//! 标量按原版 Scenario 量化：Ambient/RGB ×100（+0.01 截断），Ground/Level ×250；
//! 点光源强度 `value * 1000 + 0.1`。内部单位 `1000 == 1.0`。

use ra_assets::IniDocument;

use crate::placements::{MapEntity, MapEntityKind};

/// 内部光强单位：`1000 == 1.0`。
const LIGHT_UNIT: i32 = 1000;
/// 标量上限（对应 tint 通道乘积上限约 2.0）。
const LIGHT_CLAMP_MAX: i32 = 2000;
/// 一格边长（leptons）。
pub const LEPTONS_PER_CELL: i32 = 256;
const HALF_CELL_LEPTONS: i32 = LEPTONS_PER_CELL / 2;

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

/// 建筑点光源（灯柱 / 发光建筑等）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PointLight {
    /// 光源格 X。
    pub x: u16,
    /// 光源格 Y。
    pub y: u16,
    /// 光源中心 X（leptons）。
    pub center_x: i32,
    /// 光源中心 Y（leptons）。
    pub center_y: i32,
    /// 可见半径（leptons，含边界）。
    pub radius_leptons: i32,
    /// 强度（`1000 == 1.0`，可为负）。
    pub intensity: i32,
    /// RGB 染色（`1000 == 1.0`）。
    pub tint: [i32; 3],
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

/// 点光源 INI 浮点 → 内部单位（`*1000 + 0.1` 截断）。
pub fn light_value_to_units(value: f32) -> i32 {
    (value * LIGHT_UNIT as f32 + 0.1) as i32
}

fn signed_div_1000(value: i64) -> i32 {
    (value / i64::from(LIGHT_UNIT)) as i32
}

fn integer_sqrt(value: i64) -> i64 {
    (value as f64).sqrt() as i64
}

/// 从 rules 建筑节收集点光源（`LightIntensity≠0` 且 `LightVisibility>0`）。
pub fn collect_structure_point_lights(entities: &[MapEntity], rules: &IniDocument) -> Vec<PointLight> {
    let mut lights = Vec::new();
    for ent in entities {
        if ent.kind != MapEntityKind::Structure {
            continue;
        }
        if let Some(light) = point_light_from_rules(rules, &ent.type_id, ent.x, ent.y) {
            lights.push(light);
        }
    }
    lights
}

/// 由 rules 类型节构造格点光源。
pub fn point_light_from_rules(rules: &IniDocument, type_id: &str, x: u16, y: u16) -> Option<PointLight> {
    let intensity = rules.get(type_id, "LightIntensity").and_then(parse_f32).unwrap_or(0.0);
    let intensity_u = light_value_to_units(intensity);
    if intensity_u == 0 {
        return None;
    }
    let visibility = rules
        .get(type_id, "LightVisibility")
        .and_then(|v| v.trim().parse::<i32>().ok())
        .unwrap_or(5000)
        .max(0);
    if visibility == 0 {
        return None;
    }
    let red = rules.get(type_id, "LightRedTint").and_then(parse_f32).unwrap_or(1.0);
    let green = rules.get(type_id, "LightGreenTint").and_then(parse_f32).unwrap_or(1.0);
    let blue = rules.get(type_id, "LightBlueTint").and_then(parse_f32).unwrap_or(1.0);
    Some(PointLight {
        x,
        y,
        center_x: i32::from(x) * LEPTONS_PER_CELL + HALF_CELL_LEPTONS,
        center_y: i32::from(y) * LEPTONS_PER_CELL + HALF_CELL_LEPTONS,
        radius_leptons: visibility,
        intensity: intensity_u,
        tint: [
            light_value_to_units(red),
            light_value_to_units(green),
            light_value_to_units(blue),
        ],
    })
}

/// 手动构造测试用点光源（强度 / 染色为浮点）。
pub fn point_light_at(x: u16, y: u16, radius_leptons: i32, intensity: f32, tint: [f32; 3]) -> PointLight {
    PointLight {
        x,
        y,
        center_x: i32::from(x) * LEPTONS_PER_CELL + HALF_CELL_LEPTONS,
        center_y: i32::from(y) * LEPTONS_PER_CELL + HALF_CELL_LEPTONS,
        radius_leptons,
        intensity: light_value_to_units(intensity),
        tint: [
            light_value_to_units(tint[0]),
            light_value_to_units(tint[1]),
            light_value_to_units(tint[2]),
        ],
    }
}

/// 海拔 `z` 的环境光标量（不含点光源；`1.0` = 满亮）。
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

/// 海拔 `z` 的环境光 RGB tint（不含点光源）。
pub fn cell_tint(config: &LightingConfig, z: u8) -> [f32; 3] {
    cell_tint_with_lights(config, z, 0, 0, &[])
}

/// 地形 TMP 用地面海拔环境光（无点光源时）；有灯时请用 [`cell_tint_with_lights`] 且 `z=0`。
pub fn terrain_tint(config: &LightingConfig) -> [f32; 3] {
    cell_tint(config, 0)
}

/// 环境光 + 点光源线性衰减后的格 tint。
///
/// 衰减：`factor = (radius - distance) / radius`（距离 ≤ 半径）；强度与染色先求和再钳制。
pub fn cell_tint_with_lights(
    config: &LightingConfig,
    z: u8,
    x: u16,
    y: u16,
    lights: &[PointLight],
) -> [f32; 3] {
    let ambient = quantize(config.ambient, 100).wrapping_mul(10);
    let ground = quantize(config.ground, 250);
    let level = quantize(config.level, 250);
    let mut scalar = ambient
        .wrapping_add(level.wrapping_mul(i32::from(z)))
        .wrapping_sub(ground);
    let channel = |c: f32| quantize(c, 100).wrapping_mul(10);
    let mut rgb = [channel(config.red), channel(config.green), channel(config.blue)];

    if !lights.is_empty() {
        let cell_cx = i32::from(x) * LEPTONS_PER_CELL + HALF_CELL_LEPTONS;
        let cell_cy = i32::from(y) * LEPTONS_PER_CELL + HALF_CELL_LEPTONS;
        for light in lights {
            if light.radius_leptons <= 0 {
                continue;
            }
            let dx = i64::from(cell_cx - light.center_x);
            let dy = i64::from(cell_cy - light.center_y);
            let distance_sq = dx * dx + dy * dy;
            let radius = i64::from(light.radius_leptons);
            if distance_sq > radius * radius {
                continue;
            }
            let distance = integer_sqrt(distance_sq);
            let factor = ((radius - distance) * i64::from(LIGHT_UNIT)) / radius;
            let intensity = signed_div_1000(i64::from(light.intensity) * factor);
            scalar = scalar.wrapping_add(intensity);
            for (i, raw) in rgb.iter_mut().enumerate() {
                *raw = raw.wrapping_add(signed_div_1000(i64::from(light.tint[i]) * factor));
            }
        }
    }

    let scalar = scalar.clamp(0, LIGHT_CLAMP_MAX) as f32 / LIGHT_UNIT as f32;
    let rgb = normalize_rgb_units(rgb);
    [
        rgb[0] * scalar,
        rgb[1] * scalar,
        rgb[2] * scalar,
    ]
}

/// 最大通道归一到 `1.0`（白灯叠加后仍保持色相比例）。
fn normalize_rgb_units(rgb: [i32; 3]) -> [f32; 3] {
    let clamped = [
        rgb[0].clamp(0, LIGHT_CLAMP_MAX),
        rgb[1].clamp(0, LIGHT_CLAMP_MAX),
        rgb[2].clamp(0, LIGHT_CLAMP_MAX),
    ];
    if clamped == [LIGHT_UNIT, LIGHT_UNIT, LIGHT_UNIT] {
        return [1.0, 1.0, 1.0];
    }
    let max = clamped[0].max(clamped[1]).max(clamped[2]).max(1);
    [
        clamped[0] as f32 / max as f32,
        clamped[1] as f32 / max as f32,
        clamped[2] as f32 / max as f32,
    ]
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

    #[test]
    fn point_light_linear_falloff_on_white() {
        let cfg = LightingConfig {
            ambient: 0.5,
            ground: 0.0,
            level: 0.0,
            ..LightingConfig::identity()
        };
        let light = point_light_at(10, 10, 5 * LEPTONS_PER_CELL, 1.0, [1.0, 1.0, 1.0]);
        let lights = [light];
        let center = cell_tint_with_lights(&cfg, 0, 10, 10, &lights);
        assert!((center[0] - 1.5).abs() < 0.02, "center={}", center[0]);
        let d2 = cell_tint_with_lights(&cfg, 0, 12, 10, &lights);
        let expected = 0.5 + (5.0 - 2.0) / 5.0;
        assert!((d2[0] - expected).abs() < 0.02, "d2={} expected={}", d2[0], expected);
        let far = cell_tint_with_lights(&cfg, 0, 16, 10, &lights);
        assert!((far[0] - 0.5).abs() < 0.02, "far={}", far[0]);
    }

    #[test]
    fn point_light_edge_zero_contribution() {
        let cfg = LightingConfig {
            ambient: 0.5,
            ground: 0.0,
            level: 0.0,
            ..LightingConfig::identity()
        };
        let light = point_light_at(2, 2, 2 * LEPTONS_PER_CELL, 1.0, [1.0, 1.0, 1.0]);
        let edge = cell_tint_with_lights(&cfg, 0, 4, 2, &[light.clone()]);
        assert!((edge[0] - 0.5).abs() < 0.02);
        let inside = cell_tint_with_lights(&cfg, 0, 3, 2, &[light]);
        assert!((inside[0] - 1.0).abs() < 0.02);
    }

    #[test]
    fn negative_point_light_darkens() {
        let cfg = LightingConfig {
            ambient: 0.8,
            ground: 0.0,
            level: 0.0,
            ..LightingConfig::identity()
        };
        let light = point_light_at(2, 2, 2 * LEPTONS_PER_CELL, -0.2, [1.0, 1.0, 1.0]);
        let center = cell_tint_with_lights(&cfg, 0, 2, 2, &[light]);
        assert!((center[0] - 0.6).abs() < 0.02);
    }

    #[test]
    fn collect_structure_lights_from_rules() {
        let rules = IniDocument::parse(
            b"[LAMP]\nLightVisibility=512\nLightIntensity=0.5\nLightRedTint=1\nLightGreenTint=1\nLightBlueTint=1\n\
[DARK]\nLightVisibility=256\nLightIntensity=-0.25\n\
[ZERO]\nLightVisibility=4096\nLightIntensity=0\n",
        )
        .expect("rules");
        let entities = vec![
            MapEntity {
                kind: MapEntityKind::Structure,
                owner: "Neutral".into(),
                type_id: "LAMP".into(),
                health: 256,
                x: 3,
                y: 4,
                facing: 0,
                sub_cell: 0,
            },
            MapEntity {
                kind: MapEntityKind::Structure,
                owner: "Neutral".into(),
                type_id: "ZERO".into(),
                health: 256,
                x: 1,
                y: 1,
                facing: 0,
                sub_cell: 0,
            },
            MapEntity {
                kind: MapEntityKind::Unit,
                owner: "Americans".into(),
                type_id: "LAMP".into(),
                health: 256,
                x: 9,
                y: 9,
                facing: 0,
                sub_cell: 0,
            },
        ];
        let lights = collect_structure_point_lights(&entities, &rules);
        assert_eq!(lights.len(), 1);
        assert_eq!(lights[0].x, 3);
        assert_eq!(lights[0].y, 4);
        assert_eq!(lights[0].radius_leptons, 512);
        assert!(lights[0].intensity > 0);
    }
}
