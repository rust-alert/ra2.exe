//! 自 `engine/ra-assets/src/mission.rs` 迁出的单元测试（集成测试 crate）。

use ra_assets::mission::*;

const SAMPLE: &str = r#"
[ALL01T.MAP]
Briefing=Brief:ALL01
UIName=Name:ALL01
LSLoadMessage=LoadMsg:ALL01
LSLoadBriefing=LoadBrief:ALL01
LS640BriefLocX=20
LS640BriefLocY=20
LS800BriefLocX=24
LS800BriefLocY=28
LS640BkgdName=LS640A01.SHP
LS800BkgdName=LS800A01.SHP

[SOV01T.MAP]
Briefing=Brief:SOV01
UIName=Name:Sov01
LSLoadMessage=LoadMsg:Sov01
LSLoadBriefing=LoadBrief:Sov01
LS640BkgdName=LS640S01.SHP
LS800BkgdName=LS800S01.SHP

[Globals]
Ignore=1
"#;

#[test]
fn parses_map_sections_and_skips_non_map() {
    let missions = parse_mission_presentations(SAMPLE.as_bytes()).unwrap();
    assert_eq!(missions.len(), 2);
    assert_eq!(missions[0].scenario.as_str(), "ALL01T.MAP");
    assert_eq!(missions[0].briefing_csf.as_str(), "BRIEF:ALL01");
    assert_eq!(missions[0].ui_name_csf.as_str(), "NAME:ALL01");
    assert_eq!(missions[0].load_message_csf.as_str(), "LOADMSG:ALL01");
    assert_eq!(missions[0].load_briefing_csf.as_str(), "LOADBRIEF:ALL01");
    assert_eq!(missions[0].brief_loc_x_800, 24);
    assert_eq!(missions[0].brief_loc_y_800, 28);
    assert_eq!(missions[0].background_shp_800, "LS800A01.SHP");
    assert_eq!(missions[1].scenario.as_str(), "SOV01T.MAP");
}

#[test]
fn find_is_case_insensitive_on_scenario() {
    let missions = parse_mission_presentations(SAMPLE.as_bytes()).unwrap();
    let hit = find_mission_presentation(&missions, "all01t.map").unwrap();
    assert_eq!(hit.background_shp_for_viewport(800), Some("LS800A01.SHP"));
    assert_eq!(hit.background_shp_for_viewport(640), Some("LS640A01.SHP"));
    assert_eq!(hit.brief_loc_for_viewport(800), (24, 28));
    assert!(find_mission_presentation(&missions, "").is_none());
}
