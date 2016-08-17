//! BuildLimit：同类型存活数达上限后不可再建/再生产。

use ra_adaptor::RulesSystem;
use crate::common::battle_from_rules;
use ra_assets::{ColorSchemes, CountryRegistry, IniDocument, RulesGlobals, OverlayTypeRegistry, SuperWeaponTypeRegistry, TechnoTypeRegistry, WarheadRegistry};
use ra_engine::{BattleState, CommandRejectReason, GameCommand};
use ra_map::{MapEntity, MapEntityKind, MapInfo};
use ra_types::{GameEdition, PlayerId};

fn limit_world() -> BattleState {
    let rules_text = b"[InfantryTypes]\n0=E1\n\
[BuildingTypes]\n0=GAPILE\n\
[E1]\nStrength=125\nSpeed=4\nSight=5\nCost=200\nTechLevel=1\nOwner=Americans\nBuildLimit=1\n\
[GAPILE]\nPower=-20\nPowered=yes\nFactory=InfantryType\nOwner=Americans\nStrength=500\nSight=5\nCost=500\nTechLevel=1\n";
    let rules = IniDocument::parse(rules_text).expect("测试 INI 必须有效");
    let rules_db = RulesSystem {
        edition: GameEdition::Ra2,
        rules: rules.clone(),
        art: IniDocument::default(),
        globals: RulesGlobals::from_rules(&rules),
        overlay_types: OverlayTypeRegistry::default(),
        color_schemes: ColorSchemes::default(),
        countries: CountryRegistry::default(),
        techno_types: TechnoTypeRegistry::from_rules(&rules),
        warheads: WarheadRegistry::default(),
        super_weapons: SuperWeaponTypeRegistry::default(),
    };
    let mut map = MapInfo::empty(GameEdition::Ra2, "build-limit");
    map.width = 16;
    map.height = 16;
    map.entities = vec![
        MapEntity {
            kind: MapEntityKind::Structure,
            owner: "Americans".into(),
            type_id: "GAPILE".into(),
            health: 256,
            x: 4,
            y: 4,
            facing: 0,
            sub_cell: 0,
            mission: String::new(),
            tag: String::new(),
        },
        MapEntity {
            kind: MapEntityKind::Infantry,
            owner: "Americans".into(),
            type_id: "E1".into(),
            health: 256,
            x: 6,
            y: 4,
            facing: 0,
            sub_cell: 0,
            mission: String::new(),
            tag: String::new(),
        },
    ];
    let mut world = battle_from_rules(&rules_db, map);
    assert!(world.set_house_funds("Americans", 10_000));
    world
}

#[test]
fn produce_rejects_when_build_limit_reached() {
    let mut world = limit_world();
    world.push_command(GameCommand::Produce { player: PlayerId(0), type_id: "E1".into() });
    world.advance_tick();
    assert_eq!(world.last_rejects()[0].reason, CommandRejectReason::QueueFull);
}
