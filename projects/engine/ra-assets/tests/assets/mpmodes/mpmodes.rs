//! `mpmodes.ini` 模式表解析。

use ra_assets::parse_mpmodes;

const SAMPLE: &str = r#"
[Battle]
1=GUI:Battle, STT:ModeBattle, MPBattle.ini, standard, true

[FreeForAll]
2=GUI:FreeForAll, STT:ModeFreeForAll, MPFreeForAll.ini, standard, true

[ManBattle]
6=GUI:Duel, STT:ModeDuel, MPDuel.ini, duel, false
"#;

#[test]
fn integration_parses_and_filters_offline_skirmish() {
    let modes = parse_mpmodes(SAMPLE.as_bytes()).unwrap();
    assert_eq!(modes.len(), 3);
    assert_eq!(modes[0].id, 1);
    assert_eq!(modes[1].id, 2);
    assert_eq!(modes[2].id, 6);
    let visible: Vec<&str> = modes.iter().filter(|m| m.visible_in_offline_skirmish()).map(|m| m.name_csf.as_str()).collect();
    assert_eq!(visible, ["GUI:Battle", "GUI:FreeForAll"]);
}

// 自顶层 `mpmodes_unit.rs` 并入。

// 自 engine/ra-assets/src/mpmodes.rs :: tests

const SAMPLE_FULL: &str = r#"
; comment
[Battle]
1=GUI:Battle, STT:ModeBattle, MPBattle.ini, standard, true

[ManBattle]
5=GUI:Megawealth, STT:ModeMegawealth, MPMW.ini, megawealth, false

[FreeForAll]
2=GUI:FreeForAll, STT:ModeFreeForAll, MPFreeForAll.ini, standard, true

[Unholy]
4=GUI:UnholyAlliance, STT:ModeUnholyAlliance, MPUnholy.ini, standard, false

[Cooperative]
3=GUI:Cooperative, STT:ModeCooperative, MPCoop.ini, cooperative, false
"#;

#[test]
fn parses_stock_roster_sorted_by_id() {
    let modes = parse_mpmodes(SAMPLE_FULL.as_bytes()).unwrap();
    assert_eq!(modes.len(), 5);
    assert_eq!(modes.iter().map(|m| m.id).collect::<Vec<_>>(), vec![1, 2, 3, 4, 5]);
    assert_eq!(modes[0].name_csf, "GUI:Battle");
    assert_eq!(modes[0].map_filter, "standard");
    assert!(modes[0].random_maps_allowed);
    assert_eq!(modes[1].name_csf, "GUI:FreeForAll");
    assert_eq!(modes[1].category, "FreeForAll");
    assert!(!modes[4].random_maps_allowed);
    assert_eq!(modes[4].map_filter, "megawealth");
}

#[test]
fn offline_skirmish_visibility_matches_random_flag() {
    let modes = parse_mpmodes(SAMPLE_FULL.as_bytes()).unwrap();
    let visible: Vec<_> = modes.iter().filter(|m| m.visible_in_offline_skirmish()).map(|m| m.id).collect();
    assert_eq!(visible, vec![1, 2]);
}

#[test]
fn rejects_short_row() {
    let err = parse_mpmodes(b"[Battle]\n1=GUI:Battle, STT:ModeBattle\n").unwrap_err();
    let msg = err.to_string();
    assert!(msg.contains("至少 4"), "{msg}");
}
