//! 科技树：Owner 隔离、Prerequisite 解锁链、侧栏 Eligible 隐藏。

use crate::common::{battle_from_defs, defs_from_rules_ini};
use ra_engine::{BattleState, CommandRejectReason, GameCommand, PRODUCE_TICKS};
use ra_map::{MapEntity, MapEntityKind, MapInfo};
use ra_types::{GameEdition, PlayerId};

fn tech_defs() -> std::sync::Arc<ra_types::RuntimeDefinitions> {
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
    defs_from_rules_ini(rules_text)
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
    let mut world = battle_from_defs(GameEdition::Ra2, tech_defs(), map);
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
    world.push_command(GameCommand::Produce { player: PlayerId(0), type_id: "GAPOWR".into() });
    world.advance_tick();
    for _ in 0..=PRODUCE_TICKS {
        if world.house_ready_building("Americans").is_some() {
            break;
        }
        world.advance_tick();
    }
    world.push_command(GameCommand::PlaceBuilding { player: PlayerId(0), type_id: "GAPOWR".into(), x: 6, y: 4 });
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
    world.push_command(GameCommand::PlaceBuilding { player: PlayerId(0), type_id: "GAREFN".into(), x: 6, y: 4 });
    world.advance_tick();
    assert_eq!(world.last_rejects()[0].reason, CommandRejectReason::MissingPrerequisite);
}

#[test]
fn losing_power_hides_power_gated_buildings_again() {
    let mut world = allied_yard_world();
    world.push_command(GameCommand::Produce { player: PlayerId(0), type_id: "GAPOWR".into() });
    world.advance_tick();
    for _ in 0..=PRODUCE_TICKS {
        if world.house_ready_building("Americans").is_some() {
            break;
        }
        world.advance_tick();
    }
    world.push_command(GameCommand::PlaceBuilding { player: PlayerId(0), type_id: "GAPOWR".into(), x: 6, y: 4 });
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

// 自顶层 `gameplay__tech_tree_unit.rs` 并入。

// 自 engine/ra-engine/src/gameplay/tech_tree.rs :: tests
use std::collections::HashSet;

use ra_engine::gameplay::tech_tree::*;
use ra_types::{PrerequisiteGroups, PrerequisiteToken, RuntimeDefinitions, TechnoClass, TechnoDefinition, TypeId};

fn techno(key: &str, class: TechnoClass, owner: &str, tech_level: i32, prerequisite: &[&str], override_tokens: &[&str]) -> TechnoDefinition {
    TechnoDefinition {
        id: TypeId(1),
        type_key: key.into(),
        class,
        cost: 100,
        strength: 100,
        armor: ra_types::ArmorKind::None,
        speed: 0,
        owner: ra_types::HouseAllowList::parse_owner(owner),
        tech_level,
        naval: false,
        agent: false,
        engineer: false,
        harvester: false,
        category: String::new(),
        sight: 0,
        primary: String::new(),
        primary_id: ra_types::WeaponId(0),
        warhead: String::new(),
        warhead_id: ra_types::WarheadId(0),
        prerequisite: prerequisite.iter().filter_map(|s| PrerequisiteToken::parse_raw(s)).collect(),
        prerequisite_override: override_tokens.iter().filter_map(|s| PrerequisiteToken::parse_raw(s)).collect(),
        required_houses: ra_types::HouseAllowList::empty(),
        forbidden_houses: ra_types::HouseAllowList::empty(),
        build_limit: 0,
        build_time: 0,
        requires_stolen_allied_tech: false,
        requires_stolen_soviet_tech: false,
        requires_stolen_third_tech: false,
        pixel_selection_bracket_delta: 0,
    }
}

fn player(house: &str, tech_level: i32) -> TechTreePlayer<'_> {
    TechTreePlayer { house, tech_level, stolen_allied_tech: false, stolen_soviet_tech: false, stolen_third_tech: false }
}

fn defs_with(groups: PrerequisiteGroups, items: Vec<TechnoDefinition>) -> RuntimeDefinitions {
    let mut defs = RuntimeDefinitions { prerequisite_groups: groups, default_tech_level: 10, ..Default::default() };
    for t in items {
        defs.techno.insert(t);
    }
    defs
}

#[test]
fn empty_prerequisite_is_eligible_with_owner_and_tech() {
    let defs = defs_with(PrerequisiteGroups::default(), vec![techno("GAPOWR", TechnoClass::Building, "Americans", 1, &[], &[])]);
    let living = HashSet::new();
    assert!(is_type_eligible(&defs, player("Americans", 10), &living, "GAPOWR"));
    assert!(!is_type_eligible(&defs, player("Russians", 10), &living, "GAPOWR"));
}

#[test]
fn and_prerequisites_require_all_tokens() {
    let defs = defs_with(
        PrerequisiteGroups { power: vec!["GAPOWR".into()], ..Default::default() },
        vec![techno("GAPILE", TechnoClass::Building, "Americans", 1, &["POWER", "GAREFN"], &[])],
    );
    let mut living = HashSet::new();
    living.insert("GAPOWR".into());
    assert!(!is_type_eligible(&defs, player("Americans", 10), &living, "GAPILE"));
    living.insert("GAREFN".into());
    assert!(is_type_eligible(&defs, player("Americans", 10), &living, "GAPILE"));
}

#[test]
fn generic_power_group_or_within_list() {
    let defs = defs_with(
        PrerequisiteGroups { power: vec!["GAPOWR".into(), "NAPOWR".into()], ..Default::default() },
        vec![techno("GAREFN", TechnoClass::Building, "Americans", 1, &["POWER"], &[])],
    );
    let mut living = HashSet::new();
    living.insert("NAPOWR".into());
    assert!(is_type_eligible(&defs, player("Americans", 10), &living, "GAREFN"));
}

#[test]
fn prerequisite_override_bypasses_normal_list() {
    let defs = defs_with(PrerequisiteGroups::default(), vec![techno("SEAL", TechnoClass::Infantry, "Americans", 1, &["GATECH"], &["GACNST"])]);
    let mut living = HashSet::new();
    living.insert("GACNST".into());
    assert!(is_type_eligible(&defs, player("Americans", 10), &living, "SEAL"));
}

#[test]
fn tech_level_above_player_cap_hidden() {
    let defs = defs_with(PrerequisiteGroups::default(), vec![techno("MTNK", TechnoClass::Vehicle, "Americans", 5, &[], &[])]);
    let living = HashSet::new();
    assert!(!is_type_eligible(&defs, player("Americans", 3), &living, "MTNK"));
    assert!(is_type_eligible(&defs, player("Americans", 5), &living, "MTNK"));
}

#[test]
fn negative_tech_level_never_eligible() {
    let defs = defs_with(PrerequisiteGroups::default(), vec![techno("CIVIL", TechnoClass::Building, "", -1, &[], &[])]);
    assert!(!is_type_eligible(&defs, player("Americans", 10), &HashSet::new(), "CIVIL"));
}

#[test]
fn stolen_allied_tech_gates_eligibility() {
    let mut item = techno("SEAL", TechnoClass::Infantry, "Americans", 1, &[], &[]);
    item.requires_stolen_soviet_tech = true;
    let defs = defs_with(PrerequisiteGroups::default(), vec![item]);
    let living = HashSet::new();
    assert!(!is_type_eligible(&defs, player("Americans", 10), &living, "SEAL"));
    let unlocked =
        TechTreePlayer { house: "Americans", tech_level: 10, stolen_allied_tech: false, stolen_soviet_tech: true, stolen_third_tech: false };
    assert!(is_type_eligible(&defs, unlocked, &living, "SEAL"));
}
