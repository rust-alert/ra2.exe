//! 自 `engine/ra-map/src/structure_damage.rs` 迁出的单元测试（集成测试 crate）。

// 自 engine/ra-map/src/structure_damage.rs :: tests
use ra_assets::IniDocument;
use ra_map::structure_damage::*;

#[test]
fn parses_percent_and_picks_damaged_frame() {
    assert_eq!(parse_condition_percent("50%"), Some(0.5));
    assert_eq!(parse_condition_percent("25%"), Some(0.25));
    // 军建：黄血即受损帧。
    assert_eq!(damaged_body_frame(256, 0.5, 0.25, 1, 2), 0);
    assert_eq!(damaged_body_frame(128, 0.5, 0.25, 1, 2), 1);
    assert_eq!(damaged_body_frame(64, 0.5, 0.25, 1, 1), 0);
    // 平民：黄血不切主体，红血才切。
    assert_eq!(damaged_body_frame(128, 0.5, 0.25, -1, 2), 0);
    assert_eq!(damaged_body_frame(64, 0.5, 0.25, -1, 2), 1);
    assert_eq!(parse_damage_fire_offset("57,-13"), Some((57, -13)));
}

#[test]
fn damage_fire_types_read_from_general() {
    let doc = IniDocument::parse(
        b"[General]\nDamageFireTypes=FIRE01,FIRE02,FIRE03\n\
[AudioVisual]\nConditionYellow=50%\nConditionRed=25%\n",
    )
    .unwrap();
    let rules = StructureDamageRules::from_rules_doc(&doc);
    assert_eq!(rules.fire_types, vec!["FIRE01", "FIRE02", "FIRE03"]);
    assert_eq!(rules.yellow, 0.5);
    assert_eq!(rules.red, 0.25);
}
