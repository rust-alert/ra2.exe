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
fn wall_body_frame_uses_adjacency_and_damage_tiers() {
    // 满血独立柱。
    assert_eq!(wall_body_frame(256, 0.5, 0.25, 0, 48), 0);
    // 北+东衔接。
    assert_eq!(wall_body_frame(256, 0.5, 0.25, 1 | 2, 48), 3);
    // 四向满衔接。
    assert_eq!(wall_body_frame(256, 0.5, 0.25, 0x0F, 48), 15);
    // 黄血档：16 + mask。
    assert_eq!(wall_body_frame(128, 0.5, 0.25, 4, 48), 20);
    // 红血档：32 + mask。
    assert_eq!(wall_body_frame(64, 0.5, 0.25, 8, 48), 40);
    // 主体不足 16 帧时回退普通受损帧。
    assert_eq!(wall_body_frame(128, 0.5, 0.25, 3, 2), 1);
}

#[test]
fn wall_adjacency_mask_orthogonal_bits() {
    let cells = [(5u16, 5u16), (5, 4), (6, 5), (5, 6), (4, 5)];
    let has = |x, y| cells.contains(&(x, y));
    assert_eq!(wall_adjacency_mask(5, 5, &has), 0x0F);
    assert_eq!(wall_adjacency_mask(5, 4, &has), 4); // 仅南
    assert_eq!(wall_adjacency_mask(6, 5, &has), 8); // 仅西
    assert_eq!(wall_adjacency_mask(9, 9, &has), 0);
}

#[test]
fn damage_fire_types_read_from_general() {
    let doc = IniDocument::parse(
        b"[General]\nDamageFireTypes=FIRE01,FIRE02,FIRE03\n\
[AudioVisual]\nConditionYellow=50%\nConditionRed=25%\n",
    )
    .unwrap();
    let rules = StructureDamageRules::from_rules_doc(&doc);
    assert_eq!(
        rules.fire_types,
        vec![ra_types::ImageName::parse("FIRE01"), ra_types::ImageName::parse("FIRE02"), ra_types::ImageName::parse("FIRE03"),]
    );
    assert_eq!(rules.yellow, 0.5);
    assert_eq!(rules.red, 0.25);
}

#[test]
fn damage_rules_top_layer_overrides_underlay_thresholds() {
    let base = IniDocument::parse(b"[AudioVisual]\nConditionYellow=50%\nConditionRed=25%\n").unwrap();
    let top = IniDocument::parse(b"[AudioVisual]\nConditionYellow=40%\n").unwrap();
    let rules = StructureDamageRules::from_rules_layers(&[base, top]);
    assert_eq!(rules.yellow, 0.4);
    assert_eq!(rules.red, 0.25);
}

#[test]
fn structure_tech_level_top_layer_overrides_underlay() {
    use ra_assets::{IniMergePolicy, materialize_ini_layers};
    let base = IniDocument::parse(b"[GACNST]\nTechLevel=-1\n").unwrap();
    let top = IniDocument::parse(b"[GACNST]\nTechLevel=1\n").unwrap();
    let layers = [base, top];
    let policy = IniMergePolicy::last_wins();
    let doc = materialize_ini_layers(&layers, &policy).expect("merged");
    assert_eq!(structure_tech_level(Some(&doc), "GACNST"), 1);
}
