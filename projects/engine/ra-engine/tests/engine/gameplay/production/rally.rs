//! 工厂集结点。

use ra_adaptor::RulesDb;
use ra_assets::{ColorSchemes, IniDocument, OverlayTypeRegistry, TechnoTypeRegistry, WarheadRegistry};
use ra_engine::{CommandRejectReason, GameCommand, MatchState, PRODUCE_TICKS};
use ra_map::{MapEntity, MapEntityKind, MapInfo};
use ra_types::{EntityId, GameEdition, PlayerId};

fn barracks_world() -> MatchState {
    let rules_text = b"[InfantryTypes]\n0=E1\n\
[BuildingTypes]\n0=GAPILE\n\
[E1]\nStrength=125\nSpeed=64\nSight=5\nCost=200\n\
[GAPILE]\nPower=-20\nPowered=yes\nFactory=InfantryType\nOwner=Americans\nStrength=500\nSight=5\nCost=500\n";
    let rules = IniDocument::parse(rules_text).expect("测试 INI 必须有效");
    let rules_db = RulesDb {
        edition: GameEdition::Ra2,
        rules: rules.clone(),
        art: IniDocument::default(),
        overlay_types: OverlayTypeRegistry::default(),
        color_schemes: ColorSchemes::default(),
        techno_types: TechnoTypeRegistry::from_rules(&rules),
        warheads: WarheadRegistry::default(),
    };
    let mut map = MapInfo::empty(GameEdition::Ra2, "rally");
    map.width = 16;
    map.height = 16;
    map.entities = vec![MapEntity {
        kind: MapEntityKind::Structure,
        owner: "Americans".into(),
        type_id: "GAPILE".into(),
        health: 256,
        x: 2,
        y: 2,
        facing: 0,
        sub_cell: 0,
    }];
    let mut world = MatchState::new(GameEdition::Ra2, &rules_db, map);
    assert!(world.set_house_funds("Americans", 10_000));
    world
}

#[test]
fn set_rally_point_on_factory() {
    let mut world = barracks_world();
    world.push_command(GameCommand::SetRallyPoint { factory: EntityId(1), x: 10, y: 8 });
    world.advance_tick();
    assert!(world.last_rejects().is_empty());
    assert_eq!(world.entities[0].rally_x, Some(10));
    assert_eq!(world.entities[0].rally_y, Some(8));
}

#[test]
fn produced_unit_paths_toward_rally_point() {
    let mut world = barracks_world();
    world.push_command(GameCommand::SetRallyPoint { factory: EntityId(1), x: 10, y: 2 });
    world.advance_tick();
    world.push_command(GameCommand::Produce { player: PlayerId(0), type_id: "E1".into() });
    world.advance_tick();
    for _ in 0..(PRODUCE_TICKS - 1) {
        world.advance_tick();
    }
    assert_eq!(world.entities.len(), 2);
    let unit = &world.entities[1];
    assert_eq!(unit.type_id.as_ref(), "E1");
    assert_eq!(unit.target_x, Some(10));
    assert_eq!(unit.target_y, Some(2));
    assert!(!unit.path.is_empty());
}

#[test]
fn set_rally_rejects_non_factory() {
    let mut world = barracks_world();
    let id = world.entities[0].id;
    assert!(world.set_ecs_type_id(id, "GACNST", MapEntityKind::Structure));
    world.push_command(GameCommand::SetRallyPoint { factory: EntityId(1), x: 5, y: 5 });
    world.advance_tick();
    assert_eq!(world.last_rejects()[0].reason, CommandRejectReason::InvalidTarget);
}
