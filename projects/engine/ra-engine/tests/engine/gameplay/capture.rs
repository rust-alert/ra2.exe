//! 工程师占领可俘建筑。

use ra_adaptor::RulesSystem;
use ra_assets::{CountryRegistry, ColorSchemes, IniDocument, OverlayTypeRegistry, TechnoTypeRegistry, WarheadRegistry};
use ra_engine::{CommandRejectReason, GameCommand, BattleState};
use ra_map::{MapEntity, MapEntityKind, MapInfo};
use ra_types::GameEdition;

fn capture_rules() -> RulesSystem {
    let rules_text = b"[Countries]\n0=Americans\n1=Russians\n\
[Americans]\nSide=GDI\nMultiplay=yes\n\
[Russians]\nSide=Nod\nMultiplay=yes\n\
[General]\nPrerequisiteTech=GATECH,NATECH\n\
[InfantryTypes]\n0=ENGINEER\n1=E1\n\
[BuildingTypes]\n0=GAPOWR\n1=GATECH\n2=GACNST\n\
[ENGINEER]\nEngineer=yes\nOwner=Americans\nStrength=50\nSpeed=24\nSight=4\nCost=500\nTechLevel=1\n\
[E1]\nOwner=Americans\nStrength=125\nSpeed=24\nSight=4\nCost=200\nTechLevel=1\n\
[GAPOWR]\nPower=200\nCapturable=yes\nOwner=Americans,Russians\nStrength=750\nSight=4\nCost=800\nTechLevel=1\n\
[GATECH]\nCapturable=yes\nOwner=Americans,Russians\nStrength=500\nSight=6\nCost=2000\nTechLevel=1\n\
[GACNST]\nConstructionYard=yes\nOwner=Americans,Russians\nStrength=1000\nSight=8\nCost=2500\nTechLevel=1\n";
    let rules = IniDocument::parse(rules_text).expect("测试 INI 必须有效");
    RulesSystem {
        edition: GameEdition::Ra2,
        rules: rules.clone(),
        art: IniDocument::default(),
        overlay_types: OverlayTypeRegistry::default(),
        color_schemes: ColorSchemes::default(),
        countries: CountryRegistry::from_rules(&rules),
        techno_types: TechnoTypeRegistry::from_rules(&rules),
        warheads: WarheadRegistry::default(),
    }
}

fn capture_world(engineer_x: u16, engineer_y: u16, building_type: &str, bx: u16, by: u16) -> BattleState {
    let rules_db = capture_rules();
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
            owner: "Russians".into(),
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
    let mut world = BattleState::new(GameEdition::Ra2, &rules_db, map);
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
    assert!(world
        .last_rejects()
        .iter()
        .any(|r| r.reason == CommandRejectReason::InvalidTarget));
}

#[test]
fn engineer_captures_power_plant_and_dies() {
    let mut world = capture_world(4, 4, "GAPOWR", 5, 4);
    let engineer = world.entity_id_at(0).expect("engineer");
    let building = world.entity_id_at(1).expect("building");
    world.push_command(GameCommand::CaptureBuilding {
        engineer,
        building,
    });
    world.advance_tick();
    assert!(world.last_rejects().is_empty());
    assert!(world.ecs_health(engineer).expect("health").2);
    assert_eq!(
        world.ecs_owner(building).expect("owner").as_ref(),
        "Americans"
    );
    let ally = world
        .players
        .iter()
        .find(|p| p.house.as_ref() == "Americans")
        .expect("ally");
    let victim = world
        .players
        .iter()
        .find(|p| p.house.as_ref() == "Russians")
        .expect("victim");
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
    world.push_command(GameCommand::CaptureBuilding {
        engineer,
        building,
    });
    world.advance_tick();
    assert!(world.last_rejects().is_empty());
    assert_eq!(
        world.ecs_owner(building).expect("owner").as_ref(),
        "Americans"
    );
    let cues = world.take_eva_cues();
    assert!(cues
        .iter()
        .any(|c| c.house.as_ref() == "Americans" && c.event == "EVA_TechBuildingCaptured"));
}

#[test]
fn non_capturable_building_rejects_capture() {
    let mut world = capture_world(4, 4, "GACNST", 5, 4);
    let engineer = world.entity_id_at(0).expect("engineer");
    let building = world.entity_id_at(1).expect("building");
    world.push_command(GameCommand::CaptureBuilding { engineer, building });
    world.advance_tick();
    assert!(world
        .last_rejects()
        .iter()
        .any(|r| r.reason == CommandRejectReason::InvalidTarget));
}
