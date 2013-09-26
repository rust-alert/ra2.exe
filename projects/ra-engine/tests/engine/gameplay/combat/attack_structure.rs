//! 攻击建筑：伤害、死亡清格与电力回收。

use ra_adaptor::RulesDb;
use ra_assets::{ColorSchemes, IniDocument, OverlayTypeRegistry, TechnoTypeRegistry, WarheadRegistry};
use ra_engine::{GameCommand, MatchState};
use ra_map::{MapEntity, MapEntityKind, MapInfo};
use ra_types::{EntityId, GameEdition};

#[test]
fn attack_structure_kills_and_frees_cell() {
    let doc = IniDocument::parse(
        b"[VehicleTypes]\n0=MTNK\n\
[BuildingTypes]\n0=GAPOWR\n\
[MTNK]\nStrength=400\nSpeed=64\nSight=6\nCost=800\nArmor=none\nPrimary=Gun\n\
[GAPOWR]\nPower=200\nOwner=Americans\nStrength=80\nSight=4\nCost=600\nArmor=wood\n\
[Gun]\nDamage=40\nROF=1\nRange=6\nWarhead=SA\n\
[SA]\nVerses=100%,100%,100%,100%,100%,100%,100%,100%,100%,100%,100%\n",
    )
    .unwrap();
    let techno_types = TechnoTypeRegistry::from_rules(&doc);
    let warheads = WarheadRegistry::from_names(&doc, techno_types.iter().map(|t| t.warhead.as_str()));
    let rules = RulesDb {
        edition: GameEdition::Ra2,
        rules: doc.clone(),
        art: IniDocument::default(),
        overlay_types: OverlayTypeRegistry::default(),
        color_schemes: ColorSchemes::default(),
        techno_types,
        warheads,
    };
    let mut map = MapInfo::empty(GameEdition::Ra2, "atk-bldg");
    map.width = 16;
    map.height = 16;
    map.entities.push(MapEntity {
        kind: MapEntityKind::Unit,
        owner: "Americans".into(),
        type_id: "MTNK".into(),
        health: 256,
        x: 4,
        y: 4,
        facing: 0,
        sub_cell: 0,
    });
    map.entities.push(MapEntity {
        kind: MapEntityKind::Structure,
        owner: "Soviets".into(),
        type_id: "GAPOWR".into(),
        health: 256,
        x: 6,
        y: 4,
        facing: 0,
        sub_cell: 0,
    });
    let mut world = MatchState::new(GameEdition::Ra2, &rules, map);
    world.pass_grid.set_passable(6, 4, false);
    world.players[1].power_output = 200;
    assert!(!world.pass_grid.is_passable(6, 4));
    world.push_command(GameCommand::Attack { attacker: EntityId(1), target: EntityId(2) });
    for _ in 0..10 {
        world.advance_tick();
        if world.entities[1].dead {
            break;
        }
    }
    assert!(world.entities[1].dead);
    assert!(world.pass_grid.is_passable(6, 4));
    assert_eq!(world.players[1].power_output, 0);
}
