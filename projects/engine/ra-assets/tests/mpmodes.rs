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
    let visible: Vec<&str> = modes
        .iter()
        .filter(|m| m.visible_in_offline_skirmish())
        .map(|m| m.name_csf.as_str())
        .collect();
    assert_eq!(visible, ["GUI:Battle", "GUI:FreeForAll"]);
}
