//! 开局部队：真实 rules 下盟军池不得含苏军类型。

use crate::common::{battle_from_defs, defs_from_rules_ini};
use ra_engine::seed_skirmish_starting_units;
use ra_map::{MapInfo, Waypoint};
use ra_types::{GameEdition, TechnoClass};
use std::fs;

fn load_stock_rules() -> std::sync::Arc<ra_types::RuntimeDefinitions> {
    let path = concat!(env!("CARGO_MANIFEST_DIR"), "/../../../tmp/extract/rules.ini");
    let bytes = fs::read(path).expect("tmp/extract/rules.ini");
    defs_from_rules_ini(&bytes)
}

fn open_map() -> MapInfo {
    let mut map = MapInfo::empty(GameEdition::Ra2, "su");
    map.width = 64;
    map.height = 64;
    map.waypoints = vec![Waypoint { index: 0, x: 10, y: 10 }, Waypoint { index: 1, x: 40, y: 40 }];
    map
}

#[test]
fn americans_starting_pool_excludes_soviet_types() {
    let defs = load_stock_rules();
    let mut world = battle_from_defs(GameEdition::Ra2, defs, open_map());
    world.ensure_house("AMERICANS");
    world.set_all_players_tech_level(10);
    for y in 8..16u16 {
        for x in 8..16u16 {
            if world.pass_grid.in_bounds(x, y) {
                world.pass_grid.set_passable(x, y, true);
            }
        }
    }
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
    let defs = load_stock_rules();
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
