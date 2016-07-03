//! 科技树：Owner 隔离、Prerequisite 解锁链、侧栏 Eligible 隐藏。

use ra_adaptor::RulesSystem;
use ra_assets::{CountryRegistry, ColorSchemes, IniDocument, OverlayTypeRegistry, TechnoTypeRegistry, WarheadRegistry};
use ra_engine::{CommandRejectReason, GameCommand, BattleState, PRODUCE_TICKS};
use ra_map::{MapEntity, MapEntityKind, MapInfo};
use ra_types::{GameEdition, PlayerId};

fn tech_rules() -> RulesSystem {
    let rules_text = b"\
[General]\n\
PrerequisitePower=GAPOWR,NAPOWR\n\
PrerequisiteBarracks=GAPILE,NAHAND\n\
PrerequisiteFactory=GAWEAP,NAWEAP\n\
PrerequisiteProc=GAREFN,NAREFN\n\
PrerequisiteRadar=GAAIRC,NARADR\n\
PrerequisiteTech=GATECH,NATECH\n\
[MultiplayerDialogSettings]\nTechLevel=10\n\
[BuildingTypes]\n0=GACNST\n1=NACNST\n2=GAPOWR\n3=NAPOWR\n4=GAREFN\n5=NAREFN\n6=GAPILE\n7=NAHAND\n\
[GACNST]\nConstructionYard=yes\nOwner=Americans\nStrength=1000\nSight=8\nCost=2500\nTechLevel=1\n\
[NACNST]\nConstructionYard=yes\nOwner=Russians\nStrength=1000\nSight=8\nCost=2500\nTechLevel=1\n\
[GAPOWR]\nPower=200\nOwner=Americans\nStrength=600\nSight=4\nCost=600\nTechLevel=1\n\
[NAPOWR]\nPower=200\nOwner=Russians\nStrength=600\nSight=4\nCost=600\nTechLevel=1\n\
[GAREFN]\nPower=-50\nPowered=yes\nRefinery=yes\nOwner=Americans\nStrength=900\nSight=4\nCost=2000\nTechLevel=1\nPrerequisite=POWER\n\
[NAREFN]\nPower=-50\nPowered=yes\nRefinery=yes\nOwner=Russians\nStrength=900\nSight=4\nCost=2000\nTechLevel=1\nPrerequisite=POWER\n\
[GAPILE]\nPower=-20\nPowered=yes\nFactory=InfantryType\nOwner=Americans\nStrength=500\nSight=5\nCost=500\nTechLevel=1\nPrerequisite=POWER\n\
[NAHAND]\nPower=-20\nPowered=yes\nFactory=InfantryType\nOwner=Russians\nStrength=500\nSight=5\nCost=500\nTechLevel=1\nPrerequisite=POWER\n";
    let rules = IniDocument::parse(rules_text).expect("测试 INI 必须有效");
    RulesSystem {
        edition: GameEdition::Ra2,
        rules: rules.clone(),
        art: IniDocument::default(),
        overlay_types: OverlayTypeRegistry::default(),
        color_schemes: ColorSchemes::default(),
        countries: CountryRegistry::default(),
        techno_types: TechnoTypeRegistry::from_rules(&rules),
        warheads: WarheadRegistry::default(),
    }
}

fn allied_yard_world() -> BattleState {
    let mut map = MapInfo::empty(GameEdition::Ra2, "tech-tree");
    map.width = 16;
    map.height = 16;
    map.entities = vec![MapEntity {
        kind: MapEntityKind::Structure,
        owner: "Americans".into(),
        type_id: "GACNST".into(),
        health: 256,
        x: 4,
        y: 4,
        facing: 0,
        sub_cell: 0,
        mission: String::new(),
        tag: String::new(),
    }];
    let mut world = BattleState::new(GameEdition::Ra2, &tech_rules(), map);
    assert!(world.set_house_funds("Americans", 20_000));
    world
}

#[test]
fn allied_build_bar_hides_soviet_structures() {
    let world = allied_yard_world();
    // 经 Session 快照能力：仅本地 house。
    let session = ra_engine::Session::from_state(world, "tech");
    let caps = session.expect_battle().snapshot_capabilities(&[]);
    let ids: Vec<&str> = caps.build_items.iter().map(|i| i.type_id.as_ref()).collect();
    assert!(ids.contains(&"GAPOWR"), "空前置电厂应可见: {ids:?}");
    assert!(!ids.contains(&"NAPOWR"), "苏军电厂不应出现: {ids:?}");
    assert!(!ids.contains(&"NAHAND"), "苏军兵营不应出现: {ids:?}");
    assert!(!ids.contains(&"GAREFN"), "缺 POWER 时矿场应隐藏: {ids:?}");
    assert!(!ids.contains(&"GAPILE"), "缺 POWER 时兵营应隐藏: {ids:?}");
}

#[test]
fn power_unlocks_prerequisite_power_buildings() {
    let mut world = allied_yard_world();
    world.push_command(GameCommand::Produce {
        player: PlayerId(0),
        type_id: "GAPOWR".into(),
    });
    world.advance_tick();
    for _ in 0..=PRODUCE_TICKS {
        if world.house_ready_building("Americans").is_some() {
            break;
        }
        world.advance_tick();
    }
    world.push_command(GameCommand::PlaceBuilding {
        player: PlayerId(0),
        type_id: "GAPOWR".into(),
        x: 6,
        y: 4,
    });
    world.advance_tick();
    assert!(world.last_rejects().is_empty());

    let session = ra_engine::Session::from_state(world, "tech2");
    let caps = session.expect_battle().snapshot_capabilities(&[]);
    let ids: Vec<&str> = caps.build_items.iter().map(|i| i.type_id.as_ref()).collect();
    assert!(ids.contains(&"GAREFN"), "有 POWER 后矿场应出现: {ids:?}");
    assert!(ids.contains(&"GAPILE"), "有 POWER 后兵营应出现: {ids:?}");
    assert!(!ids.contains(&"NAREFN"), "苏军矿场仍应隐藏: {ids:?}");
}

#[test]
fn place_rejects_locked_prerequisite() {
    let mut world = allied_yard_world();
    world.push_command(GameCommand::PlaceBuilding {
        player: PlayerId(0),
        type_id: "GAREFN".into(),
        x: 6,
        y: 4,
    });
    world.advance_tick();
    assert_eq!(
        world.last_rejects()[0].reason,
        CommandRejectReason::MissingPrerequisite
    );
}

#[test]
fn losing_power_hides_power_gated_buildings_again() {
    let mut world = allied_yard_world();
    world.push_command(GameCommand::Produce {
        player: PlayerId(0),
        type_id: "GAPOWR".into(),
    });
    world.advance_tick();
    for _ in 0..=PRODUCE_TICKS {
        if world.house_ready_building("Americans").is_some() {
            break;
        }
        world.advance_tick();
    }
    world.push_command(GameCommand::PlaceBuilding {
        player: PlayerId(0),
        type_id: "GAPOWR".into(),
        x: 6,
        y: 4,
    });
    world.advance_tick();
    let power_id = world.find_entity_id_by_owner_type("Americans", "GAPOWR").expect("power");
    let max = world.ecs_health(power_id).expect("hp").1;
    assert!(world.set_ecs_health(power_id, 0, max, true));

    let session = ra_engine::Session::from_state(world, "tech3");
    let caps = session.expect_battle().snapshot_capabilities(&[]);
    let ids: Vec<&str> = caps.build_items.iter().map(|i| i.type_id.as_ref()).collect();
    assert!(!ids.contains(&"GAREFN"), "电厂被毁后矿场应再隐藏: {ids:?}");
    assert!(ids.contains(&"GAPOWR"), "电厂本身仍可重建: {ids:?}");
}
