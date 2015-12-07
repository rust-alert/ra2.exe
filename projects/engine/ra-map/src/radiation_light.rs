//! 辐射场地绿光：由站点快照 + `[Radiation]` 渲染键推导点光源。
//!
//! 不依赖战斗 sim；调用方传入当前存活站点即可。强度按 `RadLightDelay` 阶梯衰减，
//! 染色按 `remaining_at_step / duration` 比例淡出。

use ra_assets::IniDocument;

use crate::lighting::{self, PointLight, LIGHT_CLAMP_MAX, LIGHT_UNIT};

/// FNV-1a 种子（站点光 epoch）。
const FNV_OFFSET: u64 = 0xcbf2_9ce4_8422_2325;
const FNV_PRIME: u64 = 0x0000_0100_0000_01b3;

/// `[Radiation]` 中与绿光相关的键（缺节用零售缺省）。
#[derive(Debug, Clone, PartialEq)]
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
        Self {
            light_delay: 90,
            light_factor: 0.1,
            tint_factor: 1.0,
            color: (0, 255, 0),
        }
    }
}

/// 辐射站点的光照快照（由 sim 或测试构造）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
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
        Self {
            x,
            y,
            radius_leptons: radiation_site_radius_leptons(spread),
            level,
            duration,
            remaining,
        }
    }
}

/// 辐射站点落光半径（leptons）。
pub fn radiation_site_radius_leptons(spread: i32) -> i32 {
    spread.saturating_mul(lighting::LEPTONS_PER_CELL).saturating_add(lighting::LEPTONS_PER_CELL / 2)
}

/// 从 rules INI 解析 `[Radiation]` 光相关键。
pub fn parse_radiation_light_rules(doc: &IniDocument) -> RadiationLightRules {
    let mut rules = RadiationLightRules::default();
    if let Some(v) = doc.get("Radiation", "RadLightDelay").and_then(parse_i32) {
        rules.light_delay = v.max(1);
    }
    if let Some(v) = doc.get("Radiation", "RadLightFactor").and_then(parse_f32) {
        rules.light_factor = v;
    }
    if let Some(v) = doc.get("Radiation", "RadTintFactor").and_then(parse_f32) {
        rules.tint_factor = v;
    }
    if let Some(raw) = doc.get("Radiation", "RadColor") {
        if let Some(rgb) = parse_rgb_triplet(raw) {
            rules.color = rgb;
        }
    }
    rules
}

fn parse_f32(raw: &str) -> Option<f32> {
    raw.trim().parse::<f32>().ok()
}

fn parse_i32(raw: &str) -> Option<i32> {
    raw.trim().parse::<i32>().ok()
}

fn parse_rgb_triplet(raw: &str) -> Option<(u8, u8, u8)> {
    let mut it = raw.split(',').map(|p| p.trim().parse::<u8>());
    match (it.next(), it.next(), it.next()) {
        (Some(Ok(r)), Some(Ok(g)), Some(Ok(b))) => Some((r, g, b)),
        _ => None,
    }
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
    let faded = |base: i32| -> i32 {
        (i64::from(base) * i64::from(remaining_at_step) / i64::from(site.duration)) as i32
    };
    let (cr, cg, cb) = rules.color;
    let tint = [faded(channel_base(cr)), faded(channel_base(cg)), faded(channel_base(cb))];

    if intensity == 0 && tint == [0, 0, 0] {
        return None;
    }
    Some(lighting::radiation_point_light(
        site.x,
        site.y,
        site.radius_leptons,
        intensity,
        tint,
    ))
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

#[cfg(test)]
mod tests {
    use super::*;

    fn stock_rules() -> RadiationLightRules {
        RadiationLightRules::default()
    }

    fn site_at_remaining(remaining: i32) -> RadiationLightSite {
        RadiationLightSite::with_spread(100, 100, 10, 500, 500, remaining)
    }

    #[test]
    fn stock_desolator_spawn_intensity_and_pure_green_tint() {
        let light = radiation_site_light(&site_at_remaining(500), &stock_rules()).unwrap();
        assert_eq!(light.intensity, 50);
        assert_eq!(light.tint, [0, 1000, 0]);
        assert_eq!(light.radius_leptons, 2688);
    }

    #[test]
    fn stock_desolator_dual_decay_curves_are_stepwise() {
        let rules = stock_rules();
        let l1 = radiation_site_light(&site_at_remaining(410), &rules).unwrap();
        assert_eq!((l1.intensity, l1.tint[1]), (40, 820));
        let l2 = radiation_site_light(&site_at_remaining(320), &rules).unwrap();
        assert_eq!((l2.intensity, l2.tint[1]), (30, 640));
        let l5 = radiation_site_light(&site_at_remaining(50), &rules).unwrap();
        assert_eq!((l5.intensity, l5.tint[1]), (0, 100));
    }

    #[test]
    fn stepwise_holds_constant_between_step_boundaries() {
        let rules = stock_rules();
        let a = radiation_site_light(&site_at_remaining(410), &rules).unwrap();
        let b = radiation_site_light(&site_at_remaining(330), &rules).unwrap();
        assert_eq!((a.intensity, a.tint), (b.intensity, b.tint));
    }

    #[test]
    fn intensity_clamps_at_2000_when_stacked() {
        let mut site = site_at_remaining(25000);
        site.level = 25000;
        site.duration = 25000;
        site.remaining = 25000;
        let light = radiation_site_light(&site, &stock_rules()).unwrap();
        assert_eq!(light.intensity, LIGHT_CLAMP_MAX);
    }

    #[test]
    fn tint_channel_clamps_at_2000_with_high_tint_factor() {
        let mut rules = stock_rules();
        rules.tint_factor = 3.0;
        let light = radiation_site_light(&site_at_remaining(500), &rules).unwrap();
        assert_eq!(light.tint[1], LIGHT_CLAMP_MAX);
    }

    #[test]
    fn degenerate_duration_yields_no_light() {
        let mut site = site_at_remaining(0);
        site.duration = 0;
        assert!(radiation_site_light(&site, &stock_rules()).is_none());
    }

    #[test]
    fn epoch_changes_only_on_step_boundary() {
        let rules = stock_rules();
        let site_a = site_at_remaining(500);
        let e0 = radiation_light_epoch(&[site_a], &rules);
        let site_b = site_at_remaining(411); // 仍在 k=0（elapsed 89）
        assert_eq!(radiation_light_epoch(&[site_b], &rules), e0);
        let site_c = site_at_remaining(410); // k=1
        assert_ne!(radiation_light_epoch(&[site_c], &rules), e0);
    }

    #[test]
    fn parse_radiation_light_rules_reads_keys() {
        let doc = IniDocument::parse(
            b"[Radiation]\nRadLightDelay=45\nRadLightFactor=0.2\nRadTintFactor=1.5\nRadColor=10,200,30\n",
        )
        .expect("ini");
        let rules = parse_radiation_light_rules(&doc);
        assert_eq!(rules.light_delay, 45);
        assert!((rules.light_factor - 0.2).abs() < 1e-4);
        assert!((rules.tint_factor - 1.5).abs() < 1e-4);
        assert_eq!(rules.color, (10, 200, 30));
    }
}
