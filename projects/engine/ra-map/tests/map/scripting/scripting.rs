//! 自顶层 `scripting.rs`。

use ra_assets::IniDocument;
use ra_map::{MapActionKind, MapEntityKind, MapEventKind, MapInfo, parse_map_entities, parse_map_scripting};
use ra_types::GameEdition;

#[test]
fn parse_mission_and_tag_on_placements() {
    let text = b"\
[Structures]\n\
1=Neutral,GACNST,256,10,20,0,TAG01\n\
[Infantry]\n\
2=Americans,E1,256,11,21,2,Guard,32,TAG02\n\
[Units]\n\
3=Russians,HTNK,256,12,22,16,Attack,TAG03\n\
";
    let doc = IniDocument::parse(text).unwrap();
    let ents = parse_map_entities(&doc);
    let structure = ents.iter().find(|e| e.kind == MapEntityKind::Structure).unwrap();
    let infantry = ents.iter().find(|e| e.kind == MapEntityKind::Infantry).unwrap();
    let unit = ents.iter().find(|e| e.kind == MapEntityKind::Unit).unwrap();
    assert_eq!(structure.tag, "TAG01");
    assert_eq!(infantry.mission, "Guard");
    assert_eq!(infantry.facing, 32);
    assert_eq!(infantry.tag, "TAG02");
    assert_eq!(unit.mission, "Attack");
    assert_eq!(unit.tag, "TAG03");
}

#[test]
fn parse_map_scripting_triggers_and_teams() {
    let text = b"\
[Map]\nSize=0,0,10,10\nTheater=TEMPERATE\n\
[Houses]\n0=Player House\n1=BadGuy\n\
[Player House]\nCountry=Americans\nCredits=40\nPlayerControl=yes\nAllies=Player House\n\
[BadGuy]\nCountry=Russians\nCredits=20\nPlayerControl=no\n\
[Tags]\nT1=0,Start,TR1\n\
[Triggers]\nTR1=Player House,<none>,Mission Start,0,1,1,1,0\n\
[Events]\nTR1=1,13,0,0\n\
[Actions]\nTR1=1,1,0,0,0,0,0,0,A\n\
[CellTags]\n5005=T1\n\
[TaskForces]\n0=TF1\n\
[TF1]\nName=Squad\n0=2,E1\nGroup=-1\n\
[ScriptTypes]\n0=SC1\n\
[SC1]\nName=Move\n0=3,5\n\
[TeamTypes]\n0=TM1\n\
[TM1]\nName=Team\nHouse=BadGuy\nScript=SC1\nTaskForce=TF1\nWaypoint=3\nMax=1\n\
";
    let map = MapInfo::parse_ini(GameEdition::Ra2, "t.map", text).unwrap();
    assert_eq!(map.scripting.houses.len(), 2);
    assert!(map.scripting.houses[0].player_control);
    assert_eq!(map.scripting.tags[0].trigger_id, "TR1");
    assert_eq!(map.scripting.triggers[0].id, "TR1");
    assert!(!map.scripting.triggers[0].disabled);
    assert_eq!(map.scripting.events[0].conditions[0].kind, MapEventKind::TimeElapse);
    assert_eq!(map.scripting.actions[0].commands[0].kind, MapActionKind::Win);
    assert_eq!(map.scripting.cell_tags[0].x, 5);
    assert_eq!(map.scripting.cell_tags[0].y, 5);
    assert_eq!(map.scripting.task_forces[0].entries[0].type_id, "E1");
    assert_eq!(map.scripting.script_types[0].steps[0].action, 3);
    assert_eq!(map.scripting.team_types[0].task_force, "TF1");
    assert_eq!(map.scripting.team_types[0].waypoint, 3);
}

#[test]
fn parse_map_scripting_standalone() {
    let doc = IniDocument::parse(b"[Tags]\nA=2,Obj,B\n").unwrap();
    let s = parse_map_scripting(&doc);
    assert_eq!(s.tags.len(), 1);
    assert_eq!(s.tags[0].persistence, 2);
}

#[test]
fn capability_gaps_report_unsupported_actions() {
    let text = b"\
[Map]\nSize=0,0,8,8\nTheater=TEMPERATE\n\
[Triggers]\nTR1=Americans,<none>,X,0,1,1,1,0\n\
[Events]\nTR1=1,13,0,0\n\
[Actions]\nTR1=1,99,0,0,0,0,0,0,A\n\
[AITriggerTypes]\n0=AI1\n\
";
    let map = MapInfo::parse_ini(GameEdition::Ra2, "gap.map", text).unwrap();
    assert_eq!(map.scripting.ai_triggers.len(), 1);
    assert_eq!(map.scripting.ai_triggers[0].id, "AI1");
    let gaps = ra_map::map_scripting_capability_gaps(&map);
    assert!(gaps.iter().any(|g| g.code.contains("map.action.99")), "{gaps:?}");
    assert!(gaps.iter().all(|g| g.code != "map.aitrigger unsupported"), "AITriggerTypes must not block after minimal execution: {gaps:?}");
    assert!(ra_map::campaign_blocking_capability_message(&map).is_some());
}

#[test]
fn cosmetic_trigger_actions_are_supported_noops() {
    let text = b"\
[Map]\nSize=0,0,8,8\nTheater=TEMPERATE\n\
[Triggers]\nTR1=Americans,<none>,FX,0,1,1,1,0\n\
[Events]\nTR1=1,13,0,0\n\
[Actions]\nTR1=5,11,0,0,0,0,0,0,A,21,0,0,0,0,0,0,A,19,0,0,0,0,0,0,A,103,0,0,0,0,0,0,A,16,0,0,0,0,0,0,A\n\
";
    let map = MapInfo::parse_ini(GameEdition::Ra2, "fx.map", text).unwrap();
    let gaps = ra_map::map_scripting_capability_gaps(&map);
    assert!(gaps.iter().all(|g| !g.code.starts_with("map.action.")), "cosmetic actions must not block: {gaps:?}");
    assert!(ra_map::campaign_blocking_capability_message(&map).is_none());
}

#[test]
fn parse_ai_trigger_inline_csv() {
    let text = b"\
[Map]\nSize=0,0,8,8\nTheater=TEMPERATE\n\
[AITriggerTypes]\n\
AT1=Strike,TM1,Russians,1,0,GACNST,1\n\
";
    let map = MapInfo::parse_ini(GameEdition::Ra2, "ai.csv.map", text).unwrap();
    assert_eq!(map.scripting.ai_triggers.len(), 1);
    let t = &map.scripting.ai_triggers[0];
    assert_eq!(t.id, "AT1");
    assert_eq!(t.name, "Strike");
    assert_eq!(t.team, "TM1");
    assert_eq!(t.owner_house, "Russians");
    assert_eq!(t.tech_level, 1);
    assert!(map.scripting.unknown_sections.iter().all(|s| !s.eq_ignore_ascii_case("AITriggerTypes")));
}
