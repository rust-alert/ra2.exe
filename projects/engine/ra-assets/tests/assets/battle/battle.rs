//! 自 `engine/ra-assets/src/battle.rs` 迁出的单元测试（集成测试 crate）。

// 自 engine/ra-assets/src/battle.rs :: tests
use ra_assets::battle::*;

const SAMPLE: &str = r#"
[Battles]
1=TUT1
2=ALL1
3=SOV1
4=E31

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
"#;

#[test]
fn parses_stock_side_campaigns_in_list_order() {
    let camps = parse_battle_campaigns(SAMPLE.as_bytes()).unwrap();
    assert_eq!(camps.len(), 4);
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
