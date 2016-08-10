//! 自 `engine/ra-map/src/radiation_light.rs` 迁出的单元测试（集成测试 crate）。

// 自 engine/ra-map/src/radiation_light.rs :: tests
use ra_assets::IniDocument;
use ra_map::{lighting::LIGHT_CLAMP_MAX, radiation_light::*};

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
    let doc = IniDocument::parse(b"[Radiation]\nRadLightDelay=45\nRadLightFactor=0.2\nRadTintFactor=1.5\nRadColor=10,200,30\n").expect("ini");
    let rules = parse_radiation_light_rules(&doc);
    assert_eq!(rules.light_delay, 45);
    assert!((rules.light_factor - 0.2).abs() < 1e-4);
    assert!((rules.tint_factor - 1.5).abs() < 1e-4);
    assert_eq!(rules.color, (10, 200, 30));
}
