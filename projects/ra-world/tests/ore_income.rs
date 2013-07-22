//! 矿场周期采矿收入。

use ra_adaptor::RulesDb;
use ra_assets::{ColorSchemes, IniDocument, OverlayTypeRegistry, TechnoTypeRegistry};
use ra_map::{MapEntity, MapEntityKind, MapInfo};
use ra_types::GameEdition;
use ra_world::{ORE_INCOME_PER_TRIP, ORE_TRIP_TICKS, World};

fn refinery_world() -> World {
    let rules_text = b"[BuildingTypes]\n0=GAREFN\n\
[GAREFN]\nStrength=900\nSight=4\nCost=2000\n";
    let rules = IniDocument::parse(rules_text).expect("测试 INI 必须有效");
    let rules_db = RulesDb {
        edition: GameEdition::Ra2,
        rules: rules.clone(),
        art: IniDocument::default(),
        overlay_types: OverlayTypeRegistry::default(),
        color_schemes: ColorSchemes::default(),
        techno_types: TechnoTypeRegistry::from_rules(&rules),
    };
    let mut map = MapInfo::empty(GameEdition::Ra2, "ore-income");
    map.width = 8;
    map.height = 8;
    map.entities = vec![MapEntity {
        kind: MapEntityKind::Structure,
        owner: "Americans".into(),
        type_id: "GAREFN".into(),
        health: 256,
        x: 2,
        y: 2,
        facing: 0,
        sub_cell: 0,
    }];
    let mut world = World::new(GameEdition::Ra2, &rules_db, map);
    assert!(world.set_house_funds("Americans", 1_000));
    world
}

#[test]
fn living_refinery_credits_funds_each_ore_trip() {
    let mut world = refinery_world();
    for _ in 0..(ORE_TRIP_TICKS - 1) {
        world.advance_tick();
        assert_eq!(world.house_funds("Americans"), Some(1_000));
    }
    world.advance_tick();
    assert_eq!(world.house_funds("Americans"), Some(1_000 + ORE_INCOME_PER_TRIP as i32));
    for _ in 0..(ORE_TRIP_TICKS - 1) {
        world.advance_tick();
    }
    world.advance_tick();
    assert_eq!(world.house_funds("Americans"), Some(1_000 + 2 * ORE_INCOME_PER_TRIP as i32));
}

#[test]
fn dead_refinery_stops_ore_income() {
    let mut world = refinery_world();
    world.entities[0].dead = true;
    for _ in 0..ORE_TRIP_TICKS {
        world.advance_tick();
    }
    assert_eq!(world.house_funds("Americans"), Some(1_000));
}
