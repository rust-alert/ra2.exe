//! 自 `engine/ra-assets/src/rules/overlay.rs` 迁出的单元测试（集成测试 crate）。

// 自 engine/ra-assets/src/rules/overlay.rs :: tests
use ra_assets::{
    ini::IniDocument,
    rules::{color_schemes::ColorSchemes, house_remap::Hsv, overlay::*},
};

#[test]
fn marks_tiberium_flag_and_name_prefix() {
    let doc = IniDocument::parse(
        br#"
[OverlayTypes]
0=TIB01
1=BRIDGE1
2=GEM01
3=WALL1

[TIB01]
Tiberium=yes

[BRIDGE1]
Land=Road

[GEM01]
Land=Gems

[WALL1]
Land=Wall
"#,
    )
    .expect("ini");
    let reg = overlay_types_from_rules(&doc);
    assert_eq!(reg.name(0), Some("TIB01"));
    assert!(reg.is_harvestable(0));
    assert!(!reg.is_harvestable(1));
    assert!(reg.is_harvestable(2));
    assert!(!reg.is_harvestable(3));
}

#[test]
fn tiberium_overlay_hsv_maps_ore_sentinel_to_gold_and_gem_to_neon_blue() {
    let doc = IniDocument::parse(
        br#"
[Colors]
NeonGreen=0,0,0
NeonBlue=185,156,238
Gold=41,240,230

[Riparius]
Color=NeonGreen

[Cruentus]
Color=NeonBlue
"#,
    )
    .expect("ini");
    let colors = ColorSchemes::from_rules(&doc);
    let ore = tiberium_overlay_display_hsv(&doc, &colors, "TIB01").expect("ore hsv");
    assert_eq!(ore, Hsv { h: 41, s: 240, v: 230 });
    let gem = tiberium_overlay_display_hsv(&doc, &colors, "GEM01").expect("gem hsv");
    assert_eq!(gem, Hsv { h: 185, s: 156, v: 238 });
    assert!(tiberium_overlay_display_hsv(&doc, &colors, "TIBTRE01").is_none());
}
