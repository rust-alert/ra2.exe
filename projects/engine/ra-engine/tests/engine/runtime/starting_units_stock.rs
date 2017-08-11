//! 开局部队：夹具 rules 下盟军池不得含苏军类型。
//!
//! 不读 `tmp/extract`（CI / 干净工作树无原版解包）。

use crate::common::{battle_from_defs, defs_from_rules_ini};
use ra_engine::seed_skirmish_starting_units;
use ra_map::{MapInfo, Waypoint};
use ra_types::{GameEdition, TechnoClass};

fn stock_fixture_defs() -> std::sync::Arc<ra_types::RuntimeDefinitions> {
    // 与 `starting_units` 同构：盟军 E1/MTNK/MGTK，苏军 E2/HTNK，BaseUnit 为 AMCV/SMCV。
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
    let mut map = MapInfo::empty(GameEdition::Ra2, "su");
    map.width = 64;
    map.height = 64;
    map.waypoints = vec![Waypoint { index: 0, x: 10, y: 10 }, Waypoint { index: 1, x: 40, y: 40 }];
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
fn americans_starting_pool_excludes_soviet_types() {
    let defs = stock_fixture_defs();
    let mut world = battle_from_defs(GameEdition::Ra2, defs, open_map());
    world.ensure_house("AMERICANS");
    world.set_all_players_tech_level(10);
    unlock_cells(&mut world, 0, 0, 64, 64);
    let _ = world.spawn_unit_at("AMERICANS", "AMCV", 10, 10);
    let note = seed_skirmish_starting_units(&mut world, &["AMERICANS"], 10).expect("seed");
    eprintln!("seed note: {note}");
    assert!(note.contains("E1,MTNK,E1,MTNK"), "expected cheapest GI+Grizzly only: {note}");
    assert!(!note.contains("MGTK"), "Mirage must not appear: {note}");
    assert!(!note.contains("E2"), "Conscript must not appear for Allies: {note}");
    assert!(!note.contains("HTNK"), "Rhino must not appear for Allies: {note}");
    assert!(world.find_entity_id_by_owner_type("AMERICANS", "E1").is_some(), "GI not spawned");
    assert!(world.find_entity_id_by_owner_type("AMERICANS", "MTNK").is_some(), "MTNK not spawned");
    assert!(world.find_entity_id_by_owner_type("AMERICANS", "E2").is_none(), "Conscript leaked");
}

#[test]
fn dump_starting_pools_for_diagnosis() {
    let defs = stock_fixture_defs();
    let americans = defs.houses.get("Americans").expect("Americans").id;
    let russians = defs.houses.get("Russians").expect("Russians").id;
    let mut allied = Vec::new();
    let mut soviet = Vec::new();
    let mut empty_owner = Vec::new();
    for t in defs.techno.iter() {
        if !matches!(t.class, TechnoClass::Infantry | TechnoClass::Vehicle) {
            continue;
        }
        if t.naval || !t.allowed_to_start_in_multiplayer {
            continue;
        }
        let key = t.type_key.as_str();
        if t.owner_ids.is_empty() {
            empty_owner.push(format!("{key}:c{}:{:?}", t.cost, t.class));
        }
        if t.owner_ids.allows(americans) {
            allied.push(format!("{key}:c{}:{:?}", t.cost, t.class));
        }
        if t.owner_ids.allows(russians) {
            soviet.push(format!("{key}:c{}:{:?}", t.cost, t.class));
        }
    }
    allied.sort();
    soviet.sort();
    empty_owner.sort();
    eprintln!("ALLIED start-eligible ({}): {}", allied.len(), allied.join(", "));
    eprintln!("SOVIET start-eligible ({}): {}", soviet.len(), soviet.join(", "));
    eprintln!("EMPTY-OWNER start-eligible ({}): {}", empty_owner.len(), empty_owner.join(", "));
    assert!(allied.iter().any(|s| s.starts_with("E1:")), "{allied:?}");
    assert!(!allied.iter().any(|s| s.starts_with("E2:")), "E2 leaked into Allied: {allied:?}");
    assert!(!allied.iter().any(|s| s.starts_with("HTNK:")), "HTNK leaked into Allied: {allied:?}");
}
