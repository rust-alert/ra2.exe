//! 弹头 Verses 相对护甲结算伤害。

use ra_adaptor::RulesSystem;
use crate::common::battle_from_rules;
use ra_assets::{ColorSchemes, CountryRegistry, IniDocument, RulesGlobals, OverlayTypeRegistry, SuperWeaponTypeRegistry, TechnoTypeRegistry, WarheadRegistry};
use ra_engine::GameCommand;
use ra_map::{MapEntity, MapEntityKind, MapInfo};
use ra_types::{EntityId, GameEdition, TerrainSpawnerDefinitions};

#[test]
fn verses_scales_damage_against_armor() {
    let doc = IniDocument::parse(
        b"[VehicleTypes]\n0=ATK\n1=TGT\n\
[ATK]\nStrength=400\nSpeed=64\nSight=6\nCost=800\nPrimary=Gun\nArmor=none\n\
[TGT]\nStrength=400\nSpeed=0\nSight=1\nCost=100\nArmor=heavy\n\
[Gun]\nDamage=100\nROF=1\nRange=8\nWarhead=AP\n\
[AP]\nVerses=100%,100%,100%,100%,100%,50%,100%,100%,100%,100%,100%\n",
    )
    .unwrap();
    let techno_types = TechnoTypeRegistry::from_rules(&doc);
    let warheads = WarheadRegistry::from_names(&doc, techno_types.iter().map(|t| t.warhead.as_str()));
    let rules = RulesSystem {
        edition: GameEdition::Ra2,
        globals: RulesGlobals::from_rules(&doc),
        overlay_types: OverlayTypeRegistry::default(),
        terrain_spawners: TerrainSpawnerDefinitions::default(),
        color_schemes: ColorSchemes::default(),
        countries: CountryRegistry::default(),
        techno_types,
        warheads,
        super_weapons: SuperWeaponTypeRegistry::default(),
    };
    let mut map = MapInfo::empty(GameEdition::Ra2, "verses");
    map.width = 16;
    map.height = 16;
    map.entities.push(MapEntity {
        kind: MapEntityKind::Unit,
        owner: "Americans".into(),
        type_id: "ATK".into(),
        health: 256,
        x: 4,
        y: 4,
        facing: 0,
        sub_cell: 0,
        mission: String::new(),
        tag: String::new(),
    });
    map.entities.push(MapEntity {
        kind: MapEntityKind::Unit,
        owner: "Soviets".into(),
        type_id: "TGT".into(),
        health: 256,
        x: 5,
        y: 4,
        facing: 0,
        sub_cell: 0,
        mission: String::new(),
        tag: String::new(),
    });
    let mut world = battle_from_rules(&rules, map);
    assert_eq!(world.ecs_combat_view(world.entity_id_at(0).expect("entity")).expect("combat").attack_damage, 100);
    assert_eq!(world.ecs_combat_view(world.entity_id_at(0).expect("entity")).expect("combat").attack_verses[5], 50);
    assert_eq!(world.ecs_combat_view(world.entity_id_at(1).expect("entity")).expect("combat").armor, ra_types::ArmorKind::Heavy);
    world.push_command(GameCommand::Attack { attacker: EntityId(1), target: EntityId(2) });
    world.advance_tick();
    // 100 * 50% = 50
    assert_eq!(world.ecs_health(world.entity_id_at(1).expect("entity")).expect("health").0, 350);
}
