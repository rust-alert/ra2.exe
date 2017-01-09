//! 自 `engine/ra-assets/src/battle.rs` 迁出的单元测试（集成测试 crate）。

// 自 engine/ra-assets/src/battle.rs :: tests
use ra_assets::battle::*;

const SAMPLE: &str = r#"
[Battles]
1=TUT1
2=ALL1
3=SOV1
4=E31
5=TRN02
6=ALL02
7=ALL03
8=SOV02

[TUT1]
CD=0
Scenario=trn01t.MAP
Description=DESC:TUT1

[ALL1]
CD=0
Scenario=ALL01t.MAP
Description=DESC:ALL1

[SOV1]
CD=1
Scenario=SOV01t.MAP
Description=DESC:SOV1

[E31]
CD=-1
Scenario=E31.map
DebugOnly=yes
Description=DESC:E31

[TRN02]
CD=-1
Scenario=trn02t.MAP
DebugOnly=yes
Description=DESC:TRN02

[ALL02]
CD=0
Scenario=all02s.MAP
DebugOnly=yes
Description=DESC:ALL02

[ALL03]
CD=0
Scenario=all03u.MAP
DebugOnly=yes
Description=DESC:ALL03

[SOV02]
CD=1
Scenario=sov02t.MAP
DebugOnly=yes
Description=DESC:SOV02
"#;

#[test]
fn parses_stock_side_campaigns_in_list_order() {
    let camps = parse_battle_campaigns(SAMPLE.as_bytes()).unwrap();
    assert_eq!(camps.len(), 8);
    assert_eq!(camps[0].id, "TUT1");
    assert_eq!(camps[0].scenario, "trn01t.MAP");
    assert_eq!(camps[1].id, "ALL1");
    assert_eq!(camps[2].id, "SOV1");
    assert!(camps[3].debug_only);
    assert_eq!(find_battle_campaign(&camps, "all1").unwrap().description_csf, "DESC:ALL1");
}

#[test]
fn empty_without_battles_section() {
    assert!(parse_battle_campaigns(b"[ALL1]\nScenario=x.map\n").unwrap().is_empty());
}

#[test]
fn next_after_scenario_follows_same_campaign_line() {
    let camps = parse_battle_campaigns(SAMPLE.as_bytes()).unwrap();
    let next = next_battle_campaign_after_scenario(&camps, "all01t.map").expect("ALL1 next");
    assert_eq!(next.id, "ALL02");
    assert_eq!(next.scenario, "all02s.MAP");

    let next2 = next_battle_campaign_after_scenario(&camps, "ALL02s.map").expect("ALL02 next");
    assert_eq!(next2.id, "ALL03");

    let sov = next_battle_campaign_after_scenario(&camps, "sov01t.MAP").expect("SOV1 next");
    assert_eq!(sov.id, "SOV02");

    let tut = next_battle_campaign_after_scenario(&camps, "trn01t.MAP").expect("TUT1 next");
    assert_eq!(tut.id, "TRN02");

    assert!(next_battle_campaign_after_scenario(&camps, "all03u.MAP").is_none());
    assert!(next_battle_campaign_after_scenario(&camps, "missing.map").is_none());
}

#[test]
fn campaign_line_key_groups_allied_soviet_tutorial() {
    assert_eq!(campaign_line_key("ALL1"), Some("ALL"));
    assert_eq!(campaign_line_key("all12"), Some("ALL"));
    assert_eq!(campaign_line_key("SOV02"), Some("SOV"));
    assert_eq!(campaign_line_key("TUT1"), Some("TRN"));
    assert_eq!(campaign_line_key("TRN02"), Some("TRN"));
    assert_eq!(campaign_line_key("E31"), None);
}
