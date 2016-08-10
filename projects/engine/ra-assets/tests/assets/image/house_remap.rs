//! 集成测试：原 `src/house_remap.rs` 内联测试迁出。

use ra_assets::*;

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

#[test]
fn hsv_gold_is_warm() {
    let c = hsv_to_rgb(Hsv { h: 25, s: 255, v: 255 });
    assert!(c.r > c.b);
    assert!(c.g > 100);
}

#[test]
fn hsv_ramp_darkens() {
    let ramp = build_hsv_remap_ramp(Hsv { h: 0, s: 255, v: 255 });
    let lum = |c: Rgba| u16::from(c.r) + u16::from(c.g) + u16::from(c.b);
    assert!(lum(ramp[0]) > lum(ramp[15]));
}
