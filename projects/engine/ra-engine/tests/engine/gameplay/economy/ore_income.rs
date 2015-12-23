//! 矿场周期采矿收入。

use ra_adaptor::RulesSystem;
use ra_assets::{CountryRegistry, ColorSchemes, IniDocument, OverlayTypeRegistry, TechnoTypeRegistry, WarheadRegistry};
use ra_engine::{BattleState, ORE_INCOME_PER_TRIP, ORE_TRIP_TICKS};
use ra_map::{MapEntity, MapEntityKind, MapInfo};
use ra_types::GameEdition;

fn refinery_world() -> BattleState {
    let rules_text = b"[BuildingTypes]\n0=GAREFN\n\
[GAREFN]\nPower=-50\nPowered=yes\nRefinery=yes\nOwner=Americans\nStrength=900\nSight=4\nCost=2000\n";
    let rules = IniDocument::parse(rules_text).expect("测试 INI 必须有效");
    let rules_db = RulesSystem {
        edition: GameEdition::Ra2,
        rules: rules.clone(),
        art: IniDocument::default(),
        overlay_types: OverlayTypeRegistry::default(),
        color_schemes: ColorSchemes::default(),
        countries: CountryRegistry::default(),
        techno_types: TechnoTypeRegistry::from_rules(&rules),
        warheads: WarheadRegistry::default(),
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
        mission: String::new(),
        tag: String::new(),
    }];
    let mut world = BattleState::new(GameEdition::Ra2, &rules_db, map);
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
    let id = world.entity_id_at(0).expect("entity");
    let max = world.ecs_health(world.entity_id_at(0).expect("entity")).expect("health").1;
    assert!(world.set_ecs_health(id, 0, max, true));
    for _ in 0..ORE_TRIP_TICKS {
        world.advance_tick();
    }
    assert_eq!(world.house_funds("Americans"), Some(1_000));
}
