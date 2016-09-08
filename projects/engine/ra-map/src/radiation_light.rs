//! 辐射场地绿光：由站点快照 + `[Radiation]` 渲染键推导点光源。
//!
//! 不依赖战斗 sim；调用方传入当前存活站点即可。强度按 `RadLightDelay` 阶梯衰减，
//! 染色按 `remaining_at_step / duration` 比例淡出。

use ra_assets::{IniDocument, from_csv_row, parse_westwood_csv_line};
use serde::Deserialize;

use crate::lighting::{self, LIGHT_CLAMP_MAX, LIGHT_UNIT, PointLight};

/// FNV-1a 种子（站点光 epoch）。
pub const FNV_OFFSET: u64 = 0xcbf2_9ce4_8422_2325;
#[doc(hidden)]
pub const FNV_PRIME: u64 = 0x0000_0100_0000_01b3;

/// `[Radiation]` 中与绿光相关的键（缺节用零售缺省）。
#[derive(Debug, Clone, PartialEq)]
#[doc(hidden)]
pub struct RadiationLightRules {
    /// 光强阶梯间隔帧（`RadLightDelay`，缺省 90）。
    pub light_delay: i32,
    /// 每点辐射等级对应的光强（`RadLightFactor`，缺省 0.1）。
    pub light_factor: f32,
    /// 染色倍率（`RadTintFactor`，缺省 1.0）。
    pub tint_factor: f32,
    /// 发光色（`RadColor`，缺省纯绿）。
    pub color: (u8, u8, u8),
}

impl Default for RadiationLightRules {
    fn default() -> Self {
        Self { light_delay: 90, light_factor: 0.1, tint_factor: 1.0, color: (0, 255, 0) }
    }
}

/// 辐射站点的光照快照（由 sim 或测试构造）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[doc(hidden)]
pub struct RadiationLightSite {
    /// 中心格 X。
    pub x: u16,
    /// 中心格 Y。
    pub y: u16,
    /// 衰减半径（leptons；通常 `spread * 256 + 128`）。
    pub radius_leptons: i32,
    /// 站点等级（生成时光强 = level × RadLightFactor）。
    pub level: i32,
    /// 总寿命帧。
    pub duration: i32,
    /// 剩余寿命帧（≤ duration）。
    pub remaining: i32,
}

impl RadiationLightSite {
    /// 由扩散格数构造半径：`spread × 256 + 128`。
    pub fn with_spread(x: u16, y: u16, spread: i32, level: i32, duration: i32, remaining: i32) -> Self {
        Self { x, y, radius_leptons: radiation_site_radius_leptons(spread), level, duration, remaining }
    }
}

/// 辐射站点落光半径（leptons）。
pub fn radiation_site_radius_leptons(spread: i32) -> i32 {
    spread.saturating_mul(lighting::LEPTONS_PER_CELL).saturating_add(lighting::LEPTONS_PER_CELL / 2)
}

/// `[Radiation]` 光相关键（一次 Serde；缺键保持 `None`）。
#[derive(Debug, Default, Deserialize)]
struct RadiationLightSectionFields {
    #[serde(rename = "RadLightDelay")]
    light_delay: Option<i32>,
    #[serde(rename = "RadLightFactor")]
    light_factor: Option<f32>,
    #[serde(rename = "RadTintFactor")]
    tint_factor: Option<f32>,
    #[serde(rename = "RadColor", default, deserialize_with = "deserialize_optional_rgb")]
    color: Option<(u8, u8, u8)>,
}

fn deserialize_optional_rgb<'de, D>(deserializer: D) -> Result<Option<(u8, u8, u8)>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let Some(raw) = Option::<String>::deserialize(deserializer)?
    else {
        return Ok(None);
    };
    Ok(parse_rgb_triplet(&raw))
}

/// 从 rules INI 解析 `[Radiation]` 光相关键。
pub fn parse_radiation_light_rules(doc: &IniDocument) -> RadiationLightRules {
    let mut rules = RadiationLightRules::default();
    let Some(section) = doc.section("Radiation")
    else {
        return rules;
    };
    let Ok(fields) = section.deserialize::<RadiationLightSectionFields>()
    else {
        return rules;
    };
    if let Some(v) = fields.light_delay {
        rules.light_delay = v.max(1);
    }
    if let Some(v) = fields.light_factor {
        rules.light_factor = v;
    }
    if let Some(v) = fields.tint_factor {
        rules.tint_factor = v;
    }
    if let Some(rgb) = fields.color {
        rules.color = rgb;
    }
    rules
}

#[derive(Debug, Deserialize)]
struct RgbTripletRow {
    r: u8,
    g: u8,
    b: u8,
}

fn parse_rgb_triplet(raw: &str) -> Option<(u8, u8, u8)> {
    let row = from_csv_row::<RgbTripletRow>(&parse_westwood_csv_line(raw)).ok()?;
    Some((row.r, row.g, row.b))
}

/// 由单个辐射站点推导绿光；寿命无效或已完全熄灭时返回 `None`。
pub fn radiation_site_light(site: &RadiationLightSite, rules: &RadiationLightRules) -> Option<PointLight> {
    if site.duration < 1 {
        return None;
    }
    let light_delay = rules.light_delay.max(1);
    let steps_total = (site.duration / light_delay).max(1);
    let elapsed = site.duration - site.remaining;
    let k = (elapsed / light_delay).clamp(0, steps_total);
    let remaining_at_step = site.duration - k * light_delay;

    let intensity_spawn = ((site.level as f32 * rules.light_factor) as i32).min(LIGHT_CLAMP_MAX);
    let decrement = intensity_spawn / steps_total;
    let intensity = (intensity_spawn - k * decrement).max(0);

    let channel_base = |c: u8| -> i32 {
        let rescaled = (i32::from(c) * LIGHT_UNIT) / 255;
        ((rescaled as f32 * rules.tint_factor) as i32).min(LIGHT_CLAMP_MAX)
    };
    let faded = |base: i32| -> i32 { (i64::from(base) * i64::from(remaining_at_step) / i64::from(site.duration)) as i32 };
    let (cr, cg, cb) = rules.color;
    let tint = [faded(channel_base(cr)), faded(channel_base(cg)), faded(channel_base(cb))];

    if intensity == 0 && tint == [0, 0, 0] {
        return None;
    }
    Some(lighting::radiation_point_light(site.x, site.y, site.radius_leptons, intensity, tint))
}

/// 收集全部存活站点的辐射点光源。
pub fn collect_radiation_lights(sites: &[RadiationLightSite], rules: &RadiationLightRules) -> Vec<PointLight> {
    sites.iter().filter_map(|site| radiation_site_light(site, rules)).collect()
}

/// 仅在站点增删或跨越 `RadLightDelay` 阶梯时变化的廉价 epoch（供重建触发）。
pub fn radiation_light_epoch(sites: &[RadiationLightSite], rules: &RadiationLightRules) -> u64 {
    let light_delay = rules.light_delay.max(1);
    let mut h = FNV_OFFSET;
    let mut mix = |v: u64| {
        h ^= v;
        h = h.wrapping_mul(FNV_PRIME);
    };
    for site in sites {
        let steps_total = (site.duration / light_delay).max(1);
        let elapsed = site.duration - site.remaining;
        let k = (elapsed / light_delay).clamp(0, steps_total);
        mix(u64::from(site.x));
        mix(u64::from(site.y));
        mix(k as u64);
    }
    h
}
