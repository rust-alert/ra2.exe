//! 攻击建筑：伤害、死亡清格与电力回收。

use ra_adaptor::RulesSystem;
use ra_assets::{ColorSchemes, CountryRegistry, IniDocument, OverlayTypeRegistry, TechnoTypeRegistry, WarheadRegistry};
use ra_engine::{BattleState, GameCommand};
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
    let rules = RulesSystem {
        edition: GameEdition::Ra2,
        rules: doc.clone(),
        art: IniDocument::default(),
        overlay_types: OverlayTypeRegistry::default(),
        color_schemes: ColorSchemes::default(),
        countries: CountryRegistry::default(),
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
        mission: String::new(),
        tag: String::new(),
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
        mission: String::new(),
        tag: String::new(),
    });
    let mut world = BattleState::new(GameEdition::Ra2, &rules, map);
    world.pass_grid.set_passable(6, 4, false);
    world.players[1].power_output = 200;
    assert!(!world.pass_grid.is_passable(6, 4));
    world.push_command(GameCommand::Attack { attacker: EntityId(1), target: EntityId(2) });
    for _ in 0..10 {
        world.advance_tick();
        if world.ecs_health(world.entity_id_at(1).expect("entity")).expect("health").2 {
            break;
        }
    }
    assert!(world.ecs_health(world.entity_id_at(1).expect("entity")).expect("health").2);
    assert!(world.pass_grid.is_passable(6, 4));
    assert_eq!(world.players[1].power_output, 0);
}

#[test]
fn structure_damage_cues_base_under_attack_once_per_suppress_window() {
    let doc = IniDocument::parse(
        b"[VehicleTypes]\n0=MTNK\n\
[BuildingTypes]\n0=GAPOWR\n\
[MTNK]\nStrength=400\nSpeed=64\nSight=6\nCost=800\nArmor=none\nPrimary=Gun\n\
[GAPOWR]\nPower=200\nOwner=Americans\nStrength=400\nSight=4\nCost=600\nArmor=wood\n\
[Gun]\nDamage=40\nROF=1\nRange=6\nWarhead=SA\n\
[SA]\nVerses=100%,100%,100%,100%,100%,100%,100%,100%,100%,100%,100%\n",
    )
    .unwrap();
    let techno_types = TechnoTypeRegistry::from_rules(&doc);
    let warheads = WarheadRegistry::from_names(&doc, techno_types.iter().map(|t| t.warhead.as_str()));
    let rules = RulesSystem {
        edition: GameEdition::Ra2,
        rules: doc.clone(),
        art: IniDocument::default(),
        overlay_types: OverlayTypeRegistry::default(),
        color_schemes: ColorSchemes::default(),
        countries: CountryRegistry::default(),
        techno_types,
        warheads,
    };
    let mut map = MapInfo::empty(GameEdition::Ra2, "atk-base-eva");
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
        mission: String::new(),
        tag: String::new(),
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
        mission: String::new(),
        tag: String::new(),
    });
    let mut world = BattleState::new(GameEdition::Ra2, &rules, map);
    world.pass_grid.set_passable(6, 4, false);
    world.push_command(GameCommand::Attack { attacker: EntityId(1), target: EntityId(2) });

    let mut first_hit = false;
    for _ in 0..8 {
        world.advance_tick();
        let cues = world.take_eva_cues();
        let hit = cues.iter().any(|c| c.event == "EVA_OurBaseIsUnderAttack" && c.house.as_ref() == "Soviets");
        if hit {
            first_hit = true;
            break;
        }
    }
    assert!(first_hit, "first structure hit must cue EVA_OurBaseIsUnderAttack");

    let mut repeats = 0u32;
    for _ in 0..30 {
        world.advance_tick();
        let cues = world.take_eva_cues();
        if cues.iter().any(|c| c.event == "EVA_OurBaseIsUnderAttack" && c.house.as_ref() == "Soviets") {
            repeats = repeats.saturating_add(1);
        }
    }
    assert_eq!(repeats, 0, "suppress window must block repeat base-under-attack EVA");
}
