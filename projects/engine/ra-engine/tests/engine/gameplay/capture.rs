//! 工程师占领可俘建筑。

use crate::common::{battle_from_defs, defs_from_rules_ini};
use ra_engine::{BattleState, CommandRejectReason, GameCommand};
use ra_map::{MapEntity, MapEntityKind, MapInfo};
use ra_types::{GameEdition};

fn capture_defs() -> std::sync::Arc<ra_types::RuntimeDefinitions> {
        defs_from_rules_ini(b"[Countries]\n0=Americans\n1=Russians\n\
[Americans]\nSide=GDI\nMultiplay=yes\n\
[Russians]\nSide=Nod\nMultiplay=yes\n\
[General]\nPrerequisiteTech=GATECH,NATECH\n\
[InfantryTypes]\n0=ENGINEER\n1=E1\n\
[BuildingTypes]\n0=GAPOWR\n1=GATECH\n2=GACNST\n3=CAOIL\n\
[ENGINEER]\nEngineer=yes\nOwner=Americans\nStrength=50\nSpeed=24\nSight=4\nCost=500\nTechLevel=1\n\
[E1]\nOwner=Americans\nStrength=125\nSpeed=24\nSight=4\nCost=200\nTechLevel=1\n\
[GAPOWR]\nPower=200\nCapturable=yes\nOwner=Americans,Russians\nStrength=750\nSight=4\nCost=800\nTechLevel=1\n\
[GATECH]\nCapturable=yes\nOwner=Americans,Russians,Neutral\nStrength=500\nSight=6\nCost=2000\nTechLevel=1\n\
[GACNST]\nConstructionYard=yes\nOwner=Americans,Russians\nStrength=1000\nSight=8\nCost=2500\nTechLevel=1\n\
[CAOIL]\nCapturable=yes\nFoundation=2x2\nOwner=Americans,Russians,Neutral\nStrength=800\nSight=4\nCost=1500\nTechLevel=1\n")
}

fn capture_world(engineer_x: u16, engineer_y: u16, building_type: &str, bx: u16, by: u16) -> BattleState {
    capture_world_owned(engineer_x, engineer_y, building_type, bx, by, "Russians")
}

fn capture_world_owned(engineer_x: u16, engineer_y: u16, building_type: &str, bx: u16, by: u16, building_owner: &str) -> BattleState {
    let defs = capture_defs();
    let mut map = MapInfo::empty(GameEdition::Ra2, "engineer-capture");
    map.width = 16;
    map.height = 16;
    map.entities = vec![
        MapEntity {
            kind: MapEntityKind::Infantry,
            owner: "Americans".into(),
            type_id: "ENGINEER".into(),
            health: 256,
            x: engineer_x,
            y: engineer_y,
            facing: 0,
            sub_cell: 0,
            mission: String::new(),
            tag: String::new(),
        },
        MapEntity {
            kind: MapEntityKind::Structure,
            owner: building_owner.into(),
            type_id: building_type.into(),
            health: 256,
            x: bx,
            y: by,
            facing: 0,
            sub_cell: 0,
            mission: String::new(),
            tag: String::new(),
        },
        MapEntity {
            kind: MapEntityKind::Structure,
            owner: "Americans".into(),
            type_id: "GACNST".into(),
            health: 256,
            x: 1,
            y: 1,
            facing: 0,
            sub_cell: 0,
            mission: String::new(),
            tag: String::new(),
        },
    ];
    let mut world = battle_from_defs(GameEdition::Ra2, defs, map);
    world.set_all_players_funds(10_000);
    if let Some(p) = world.players.iter_mut().find(|p| p.house.as_ref() == "Russians") {
        p.power_output = 200;
        p.power_drain = 50;
    }
    if let Some(p) = world.players.iter_mut().find(|p| p.house.as_ref() == "Americans") {
        p.power_output = 0;
        p.power_drain = 0;
    }
    world
}

#[test]
fn non_engineer_cannot_capture() {
    let mut world = capture_world(4, 4, "GAPOWR", 5, 4);
    let engineer = world.entity_id_at(0).expect("engineer");
    assert!(world.set_ecs_type_id(engineer, "E1", MapEntityKind::Infantry));
    let building = world.entity_id_at(1).expect("building");
    world.push_command(GameCommand::CaptureBuilding { engineer, building });
    world.advance_tick();
    assert!(world.last_rejects().iter().any(|r| r.reason == CommandRejectReason::InvalidTarget));
}

#[test]
fn engineer_captures_power_plant_and_dies() {
    let mut world = capture_world(4, 4, "GAPOWR", 5, 4);
    let engineer = world.entity_id_at(0).expect("engineer");
    let building = world.entity_id_at(1).expect("building");
    world.push_command(GameCommand::CaptureBuilding { engineer, building });
    world.advance_tick();
    assert!(world.last_rejects().is_empty());
    assert!(world.ecs_health(engineer).expect("health").2);
    assert_eq!(world.ecs_owner(building).expect("owner").as_ref(), "Americans");
    let ally = world.players.iter().find(|p| p.house.as_ref() == "Americans").expect("ally");
    let victim = world.players.iter().find(|p| p.house.as_ref() == "Russians").expect("victim");
    assert_eq!(ally.power_output, 200);
    assert_eq!(victim.power_output, 0);
    assert_eq!(world.take_structure_paint_dirty(), vec![building]);
    let cues = world.take_eva_cues();
    assert!(cues.iter().any(|c| c.house.as_ref() == "Americans" && c.event == "EVA_BuildingCaptured"));
    assert!(cues.iter().any(|c| c.house.as_ref() == "Russians" && c.event == "EVA_BuildingCaptured"));
}

#[test]
fn engineer_capturing_tech_building_emits_tech_eva() {
    let mut world = capture_world(4, 4, "GATECH", 5, 4);
    let engineer = world.entity_id_at(0).expect("engineer");
    let building = world.entity_id_at(1).expect("building");
    world.push_command(GameCommand::CaptureBuilding { engineer, building });
    world.advance_tick();
    assert!(world.last_rejects().is_empty());
    assert_eq!(world.ecs_owner(building).expect("owner").as_ref(), "Americans");
    let cues = world.take_eva_cues();
    assert!(cues.iter().any(|c| c.house.as_ref() == "Americans" && c.event == "EVA_TechBuildingCaptured"));
}

#[test]
fn non_capturable_building_rejects_capture() {
    let mut world = capture_world(4, 4, "GACNST", 5, 4);
    let engineer = world.entity_id_at(0).expect("engineer");
    let building = world.entity_id_at(1).expect("building");
    world.push_command(GameCommand::CaptureBuilding { engineer, building });
    world.advance_tick();
    assert!(world.last_rejects().iter().any(|r| r.reason == CommandRejectReason::InvalidTarget));
}

#[test]
fn engineer_captures_when_adjacent_to_2x2_footprint_edge() {
    // CAOIL 2x2 锚点 (4,4)；工程师站在 (6,4) 对锚点曼哈顿=2，但对外沿邻接。
    let mut world = capture_world(6, 4, "CAOIL", 4, 4);
    let building = world.entity_id_at(1).expect("building");
    let engineer = world.entity_id_at(0).expect("engineer");
    world.push_command(GameCommand::CaptureBuilding { engineer, building });
    world.advance_tick();
    assert!(world.last_rejects().is_empty());
    assert_eq!(world.ecs_owner(building).expect("owner").as_ref(), "Americans");
    assert!(world.ecs_health(engineer).expect("health").2);
}

#[test]
fn engineer_captures_neutral_tech_building_without_victim_eva() {
    let mut world = capture_world_owned(4, 4, "GATECH", 5, 4, "Neutral");
    let building = world.entity_id_at(1).expect("building");
    let engineer = world.entity_id_at(0).expect("engineer");
    world.push_command(GameCommand::CaptureBuilding { engineer, building });
    world.advance_tick();
    assert!(world.last_rejects().is_empty());
    assert_eq!(world.ecs_owner(building).expect("owner").as_ref(), "Americans");
    let cues = world.take_eva_cues();
    assert!(cues.iter().any(|c| c.house.as_ref() == "Americans" && c.event == "EVA_TechBuildingCaptured"));
    assert!(!cues.iter().any(|c| c.house.as_ref().eq_ignore_ascii_case("Neutral")));
}
