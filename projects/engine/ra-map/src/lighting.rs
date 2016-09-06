//! 地图 `[Lighting]`：环境光、Ion 档、点光源与每格 RGB tint。
//!
//! 标量按原版 Scenario 量化：Ambient/RGB ×100（+0.01 截断），Ground/Level ×250；
//! 点光源强度 `value * 1000 + 0.1`。内部单位 `1000 == 1.0`。
//!
//! Ion 档（闪电风暴 / 离子风暴）读 `IonAmbient` / `IonRed` / …；缺键用零售缺省
//! （Ambient≈0.87、偏蓝紫通道、Ground/Level=0）。

use ra_assets::IniDocument;
use serde::Deserialize;

use crate::placements::{MapEntity, MapEntityKind};

/// 内部光强单位：`1000 == 1.0`。
pub const LIGHT_UNIT: i32 = 1000;
/// 标量上限（对应 tint 通道乘积上限约 2.0）。
pub const LIGHT_CLAMP_MAX: i32 = 2000;
/// 一格边长（leptons）。
pub const LEPTONS_PER_CELL: i32 = 256;
#[doc(hidden)]
pub const HALF_CELL_LEPTONS: i32 = LEPTONS_PER_CELL / 2;

/// 当前生效的环境光档（普通 / Ion 风暴）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
#[doc(hidden)]
pub enum LightingProfile {
    /// `[Lighting]` Ambient/RGB/Ground/Level。
    #[default]
    Normal,
    /// `[Lighting]` IonAmbient / IonRed / …（闪电风暴目标档）。
    Ion,
}

/// 地图 `[Lighting]` 一组环境光参数（普通或 Ion）。
#[derive(Debug, Clone, Copy, PartialEq)]
#[doc(hidden)]
pub struct LightingConfig {
    /// 基础亮度（普通缺省 `1.0`；Ion 缺省 `0.87`）。
    pub ambient: f32,
    /// 红通道倍率。
    pub red: f32,
    /// 绿通道倍率。
    pub green: f32,
    /// 蓝通道倍率。
    pub blue: f32,
    /// 地面压暗项（普通缺省 `0.20`；Ion 缺省 `0.0`）。
    pub ground: f32,
    /// 每级海拔对 ambient 的增量（普通缺省 `0.032`；Ion 缺省 `0.0`）。
    pub level: f32,
}

impl Default for LightingConfig {
    fn default() -> Self {
        Self { ambient: 1.0, red: 1.0, green: 1.0, blue: 1.0, ground: 0.20, level: 0.032 }
    }
}

impl LightingConfig {
    /// 无压暗、通道为 1 的恒等配置（测试用；零售缺省仍含 `Ground=0.20`）。
    pub const fn identity() -> Self {
        Self { ambient: 1.0, red: 1.0, green: 1.0, blue: 1.0, ground: 0.0, level: 0.0 }
    }

    /// 闪电风暴 / Ion 零售缺省档。
    pub const fn ion_default() -> Self {
        Self { ambient: 0.87, red: 0.30, green: 0.40, blue: 0.75, ground: 0.0, level: 0.0 }
    }
}

/// 地图解析出的普通 + Ion 两套环境光。
#[derive(Debug, Clone, Copy, PartialEq)]
#[doc(hidden)]
pub struct MapLightingProfiles {
    /// 日常档。
    pub normal: LightingConfig,
    /// Ion / 闪电风暴档。
    pub ion: LightingConfig,
}

impl Default for MapLightingProfiles {
    fn default() -> Self {
        Self { normal: LightingConfig::default(), ion: LightingConfig::ion_default() }
    }
}

impl MapLightingProfiles {
    /// 按当前档取环境光。
    pub fn config(&self, profile: LightingProfile) -> LightingConfig {
        match profile {
            LightingProfile::Normal => self.normal,
            LightingProfile::Ion => self.ion,
        }
    }
}

/// 建筑点光源（灯柱 / 发光建筑等）。
#[derive(Debug, Clone, PartialEq, Eq)]
#[doc(hidden)]
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

/// `[Lighting]` 节字段（一次 Serde；缺键保持 `None`，由缺省档填充）。
#[derive(Debug, Default, Deserialize)]
struct LightingSectionFields {
    #[serde(rename = "Ambient")]
    ambient: Option<f32>,
    #[serde(rename = "Red")]
    red: Option<f32>,
    #[serde(rename = "Green")]
    green: Option<f32>,
    #[serde(rename = "Blue")]
    blue: Option<f32>,
    #[serde(rename = "Ground")]
    ground: Option<f32>,
    #[serde(rename = "Level")]
    level: Option<f32>,
    #[serde(rename = "IonAmbient")]
    ion_ambient: Option<f32>,
    #[serde(rename = "IonRed")]
    ion_red: Option<f32>,
    #[serde(rename = "IonGreen")]
    ion_green: Option<f32>,
    #[serde(rename = "IonBlue")]
    ion_blue: Option<f32>,
    #[serde(rename = "IonGround")]
    ion_ground: Option<f32>,
    #[serde(rename = "IonLevel")]
    ion_level: Option<f32>,
}

impl LightingSectionFields {
    fn has_normal(&self) -> bool {
        self.ambient.is_some()
            || self.red.is_some()
            || self.green.is_some()
            || self.blue.is_some()
            || self.ground.is_some()
            || self.level.is_some()
    }

    fn has_ion(&self) -> bool {
        self.ion_ambient.is_some()
            || self.ion_red.is_some()
            || self.ion_green.is_some()
            || self.ion_blue.is_some()
            || self.ion_ground.is_some()
            || self.ion_level.is_some()
    }

    fn apply_normal(&self, cfg: &mut LightingConfig) {
        if let Some(v) = self.ambient {
            cfg.ambient = v;
        }
        if let Some(v) = self.red {
            cfg.red = v;
        }
        if let Some(v) = self.green {
            cfg.green = v;
        }
        if let Some(v) = self.blue {
            cfg.blue = v;
        }
        if let Some(v) = self.ground {
            cfg.ground = v;
        }
        if let Some(v) = self.level {
            cfg.level = v;
        }
    }

    fn apply_ion(&self, cfg: &mut LightingConfig) {
        if let Some(v) = self.ion_ambient {
            cfg.ambient = v;
        }
        if let Some(v) = self.ion_red {
            cfg.red = v;
        }
        if let Some(v) = self.ion_green {
            cfg.green = v;
        }
        if let Some(v) = self.ion_blue {
            cfg.blue = v;
        }
        if let Some(v) = self.ion_ground {
            cfg.ground = v;
        }
        if let Some(v) = self.ion_level {
            cfg.level = v;
        }
    }
}

/// 从地图 INI 解析 `[Lighting]` 普通档；缺节或缺键用零售缺省。
///
/// 仅读 Ambient/RGB/Ground/Level。Ion 档请用 [`parse_map_lighting`]。
pub fn parse_lighting(doc: &IniDocument) -> LightingConfig {
    parse_map_lighting(doc).normal
}

/// 从地图 INI 解析普通 + Ion 两套环境光。
pub fn parse_map_lighting(doc: &IniDocument) -> MapLightingProfiles {
    let mut out = MapLightingProfiles::default();
    let Some(section) = doc.section("Lighting")
    else {
        return out;
    };
    let Ok(fields) = section.deserialize::<LightingSectionFields>()
    else {
        return out;
    };
    if !fields.has_normal() && !fields.has_ion() {
        return out;
    }
    if fields.has_normal() {
        fields.apply_normal(&mut out.normal);
    }
    if fields.has_ion() {
        fields.apply_ion(&mut out.ion);
    }
    out
}

#[doc(hidden)]
pub fn quantize(value: f32, scale: i32) -> i32 {
    (f64::from(value) * f64::from(scale) + 0.01) as i32
}

/// 点光源 INI 浮点 → 内部单位（`*1000 + 0.1` 截断）。
pub fn light_value_to_units(value: f32) -> i32 {
    (value * LIGHT_UNIT as f32 + 0.1) as i32
}

#[doc(hidden)]
pub fn signed_div_1000(value: i64) -> i32 {
    (value / i64::from(LIGHT_UNIT)) as i32
}

#[doc(hidden)]
pub fn integer_sqrt(value: i64) -> i64 {
    (value as f64).sqrt() as i64
}

/// 从冻结建筑光表收集点光源。
pub fn collect_structure_point_lights(entities: &[MapEntity], lights: &StructureLightTable) -> Vec<PointLight> {
    let mut out = Vec::new();
    for ent in entities {
        if ent.kind != MapEntityKind::Structure {
            continue;
        }
        if let Some(light) = point_light_from_profile(lights.get(&ent.type_id), ent.x, ent.y) {
            out.push(light);
        }
    }
    out
}

/// 由冻结光资料构造格点光源。
pub fn point_light_from_profile(profile: Option<&ra_types::StructureLightProfile>, x: u16, y: u16) -> Option<PointLight> {
    let profile = profile?;
    if profile.intensity == 0 || profile.radius_leptons <= 0 {
        return None;
    }
    Some(PointLight {
        x,
        y,
        center_x: i32::from(x) * LEPTONS_PER_CELL + HALF_CELL_LEPTONS,
        center_y: i32::from(y) * LEPTONS_PER_CELL + HALF_CELL_LEPTONS,
        radius_leptons: profile.radius_leptons,
        intensity: profile.intensity,
        tint: profile.tint,
    })
}

/// 类型键 → 建筑点光源（大写键）。
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct StructureLightTable {
    by_key: std::collections::BTreeMap<String, ra_types::StructureLightProfile>,
}

impl StructureLightTable {
    /// 从冻结建筑表投影发光条目。
    pub fn from_structures(structures: &ra_types::StructureDefinitions) -> Self {
        let mut by_key = std::collections::BTreeMap::new();
        for s in structures.iter() {
            if let Some(light) = s.light {
                by_key.insert(s.type_key.clone(), light);
            }
        }
        Self { by_key }
    }

    /// 插入或覆盖一条类型光资料（键转大写）。
    pub fn insert(&mut self, type_key: impl Into<String>, profile: ra_types::StructureLightProfile) {
        self.by_key.insert(type_key.into().to_ascii_uppercase(), profile);
    }

    /// 按类型键查找。
    pub fn get(&self, type_key: &str) -> Option<&ra_types::StructureLightProfile> {
        self.by_key.get(&type_key.to_ascii_uppercase())
    }
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
        tint: [light_value_to_units(tint[0]), light_value_to_units(tint[1]), light_value_to_units(tint[2])],
    }
}

/// 辐射绿光点光源（强度 / 染色已是内部单位 `1000 == 1.0`）。
pub fn radiation_point_light(x: u16, y: u16, radius_leptons: i32, intensity: i32, tint: [i32; 3]) -> PointLight {
    PointLight {
        x,
        y,
        center_x: i32::from(x) * LEPTONS_PER_CELL + HALF_CELL_LEPTONS,
        center_y: i32::from(y) * LEPTONS_PER_CELL + HALF_CELL_LEPTONS,
        radius_leptons: radius_leptons.max(0),
        intensity,
        tint,
    }
}

/// 海拔 `z` 的环境光标量（不含点光源；`1.0` = 满亮）。
pub fn cell_light_scalar(config: &LightingConfig, z: u8) -> f32 {
    let ambient = quantize(config.ambient, 100).wrapping_mul(10);
    let ground = quantize(config.ground, 250);
    let level = quantize(config.level, 250);
    let scalar = ambient.wrapping_add(level.wrapping_mul(i32::from(z))).wrapping_sub(ground).clamp(0, LIGHT_CLAMP_MAX);
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
pub fn cell_tint_with_lights(config: &LightingConfig, z: u8, x: u16, y: u16, lights: &[PointLight]) -> [f32; 3] {
    let ambient = quantize(config.ambient, 100).wrapping_mul(10);
    let ground = quantize(config.ground, 250);
    let level = quantize(config.level, 250);
    let mut scalar = ambient.wrapping_add(level.wrapping_mul(i32::from(z))).wrapping_sub(ground);
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
    [rgb[0] * scalar, rgb[1] * scalar, rgb[2] * scalar]
}

/// 最大通道归一到 `1.0`（白灯叠加后仍保持色相比例）。
pub fn normalize_rgb_units(rgb: [i32; 3]) -> [f32; 3] {
    let clamped = [rgb[0].clamp(0, LIGHT_CLAMP_MAX), rgb[1].clamp(0, LIGHT_CLAMP_MAX), rgb[2].clamp(0, LIGHT_CLAMP_MAX)];
    if clamped == [LIGHT_UNIT, LIGHT_UNIT, LIGHT_UNIT] {
        return [1.0, 1.0, 1.0];
    }
    let max = clamped[0].max(clamped[1]).max(clamped[2]).max(1);
    [clamped[0] as f32 / max as f32, clamped[1] as f32 / max as f32, clamped[2] as f32 / max as f32]
}

/// 就地乘 RGB tint；`alpha` 不变。
pub fn apply_rgba_tint(rgba: &mut [u8], tint: [f32; 3]) {
    for px in rgba.chunks_exact_mut(4) {
        px[0] = mul_channel(px[0], tint[0]);
        px[1] = mul_channel(px[1], tint[1]);
        px[2] = mul_channel(px[2], tint[2]);
    }
}

#[doc(hidden)]
pub fn mul_channel(value: u8, tint: f32) -> u8 {
    (f32::from(value) * tint).clamp(0.0, 255.0) as u8
}
