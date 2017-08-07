//! 开局部队组成与占位：阵营 Owner、最便宜兵种、避开建造场占地。

use crate::common::{battle_from_defs, defs_from_rules_ini};
use ra_engine::seed_skirmish_starting_units;
use ra_map::{MapInfo, Waypoint};
use ra_types::GameEdition;

fn start_defs() -> std::sync::Arc<ra_types::RuntimeDefinitions> {
    defs_from_rules_ini(
        b"[General]\nBaseUnit=AMCV,SMCV\n\
[VehicleTypes]\n0=AMCV\n1=SMCV\n2=MTNK\n3=HTNK\n4=MGTK\n\
[InfantryTypes]\n0=E1\n1=E2\n\
[BuildingTypes]\n0=GACNST\n1=NACNST\n\
[AMCV]\nDeploysInto=GACNST\nOwner=Americans\nStrength=1000\nSpeed=32\nSight=4\nCost=2500\nTechLevel=1\n\
[SMCV]\nDeploysInto=NACNST\nOwner=Russians\nStrength=1000\nSpeed=32\nSight=4\nCost=2500\nTechLevel=1\n\
[MTNK]\nOwner=Americans\nStrength=400\nSpeed=6\nSight=6\nCost=700\nTechLevel=2\n\
[MGTK]\nOwner=Americans\nStrength=200\nSpeed=7\nSight=9\nCost=1000\nTechLevel=9\n\
[HTNK]\nOwner=Russians\nStrength=600\nSpeed=4\nSight=6\nCost=900\nTechLevel=2\n\
[E1]\nOwner=Americans\nStrength=125\nSpeed=4\nSight=5\nCost=200\nTechLevel=1\n\
[E2]\nOwner=Russians\nStrength=125\nSpeed=4\nSight=5\nCost=100\nTechLevel=1\n\
[GACNST]\nConstructionYard=yes\nOwner=Americans\nStrength=1000\nSight=8\nCost=2500\nFoundation=4x4\n\
[NACNST]\nConstructionYard=yes\nOwner=Russians\nStrength=1000\nSight=8\nCost=2500\nFoundation=4x4\n",
    )
}

fn open_map() -> MapInfo {
    let mut map = MapInfo::empty(GameEdition::Ra2, "start-units");
    map.width = 48;
    map.height = 48;
    map.waypoints = vec![Waypoint { index: 0, x: 12, y: 12 }, Waypoint { index: 1, x: 30, y: 30 }];
    map
}

fn unlock_cells(world: &mut ra_engine::BattleState, x0: u16, y0: u16, w: u16, h: u16) {
    for y in y0..(y0 + h) {
        for x in x0..(x0 + w) {
            if world.pass_grid.in_bounds(x, y) {
                world.pass_grid.set_passable(x, y, true);
            }
        }
    }
}

#[test]
fn compose_repeats_cheapest_only_not_high_tech() {
    let defs = start_defs();
    let mut world = battle_from_defs(GameEdition::Ra2, defs, open_map());
    world.ensure_house("AMERICANS");
    world.set_all_players_tech_level(10);
    unlock_cells(&mut world, 0, 0, 48, 48);
    let _ = world.spawn_unit_at("AMERICANS", "AMCV", 12, 12);
    let note = seed_skirmish_starting_units(&mut world, &["AMERICANS"], 8).expect("seed");
    assert!(note.contains("E1,MTNK,E1,MTNK,E1,MTNK,E1,MTNK"), "got {note}");
    assert!(!note.contains("MGTK"), "high-tech must not enter Unit Count plan: {note}");
    assert!(!note.contains("E2"), "Soviet infantry must not enter Allied plan: {note}");
    assert!(!note.contains("HTNK"), "Soviet tank must not enter Allied plan: {note}");
}

#[test]
fn allied_seed_stays_outside_yard_foundation() {
    let defs = start_defs();
    let mut world = battle_from_defs(GameEdition::Ra2, defs, open_map());
    world.ensure_house("AMERICANS");
    world.set_all_players_tech_level(10);
    unlock_cells(&mut world, 0, 0, 48, 48);
    let mcv = world.spawn_unit_at("AMERICANS", "AMCV", 12, 12).expect("mcv");
    let note = seed_skirmish_starting_units(&mut world, &["AMERICANS"], 10).expect("seed");
    assert!(note.contains("n10/10"), "all ten should place: {note}");

    // 4×4 建造场占地 [12,16)×[12,16) 内不得有开局部队（仅允许 MCV 锚点）。
    for id in world.entity_ids() {
        if id == mcv {
            continue;
        }
        let Some((ty, _)) = world.ecs_identity(id)
        else {
            continue;
        };
        if !matches!(ty.as_ref(), "E1" | "MTNK") {
            continue;
        }
        let (x, y, _) = world.ecs_transform(id).expect("xf");
        let inside = x >= 12 && x < 16 && y >= 12 && y < 16;
        assert!(!inside, "{ty} spawned inside yard footprint at ({x},{y})");
    }
}

#[test]
fn russians_get_conscript_and_rhino_not_allied() {
    let defs = start_defs();
    let mut world = battle_from_defs(GameEdition::Ra2, defs, open_map());
    world.ensure_house("RUSSIANS");
    world.set_all_players_tech_level(10);
    unlock_cells(&mut world, 0, 0, 48, 48);
    let _ = world.spawn_unit_at("RUSSIANS", "SMCV", 30, 30);
    let note = seed_skirmish_starting_units(&mut world, &["", "RUSSIANS"], 4).expect("seed");
    // 席位 0 为空，席位 1 = 俄军 @ 航点 1
    assert!(note.contains("RUSSIANS@1:"), "{note}");
    assert!(note.contains("E2,HTNK,E2,HTNK"), "got {note}");
    assert!(!note.contains("E1"), "{note}");
    assert!(!note.contains("MTNK"), "{note}");
}
