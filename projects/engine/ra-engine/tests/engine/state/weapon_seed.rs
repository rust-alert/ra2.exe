//! 主武器 Damage / Range 播种到世界实体。

use ra_adaptor::RulesSystem;
use crate::common::battle_from_rules;
use ra_assets::{ColorSchemes, CountryRegistry, IniDocument, RulesGlobals, OverlayTypeRegistry, SuperWeaponTypeRegistry, TechnoTypeRegistry, WarheadRegistry};
use ra_map::{MapEntity, MapEntityKind, MapInfo};
use ra_types::{GameEdition, TerrainSpawnerDefinitions};

#[test]
fn seeds_attack_stats_from_primary_weapon() {
    let doc = IniDocument::parse(
        b"[VehicleTypes]\n0=MTNK\n\
[MTNK]\nStrength=400\nSpeed=64\nSight=6\nCost=800\nPrimary=90mm\n\
[90mm]\nDamage=75\nROF=20\nRange=5\n",
    )
    .unwrap();
    let rules = RulesSystem {
        edition: GameEdition::Ra2,
        rules: doc.clone(),
        art: IniDocument::default(),
        globals: RulesGlobals::from_rules(&doc),
        overlay_types: OverlayTypeRegistry::default(),
        terrain_spawners: TerrainSpawnerDefinitions::default(),
        color_schemes: ColorSchemes::default(),
        countries: CountryRegistry::default(),
        techno_types: TechnoTypeRegistry::from_rules(&doc),
        warheads: WarheadRegistry::default(),
        super_weapons: SuperWeaponTypeRegistry::default(),
    };
    let mut map = MapInfo::empty(GameEdition::Ra2, "weapon");
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
    let world = battle_from_rules(&rules, map);
    let combat = world.ecs_combat_view(world.entity_id_at(0).expect("entity")).expect("combat");
    assert_eq!(combat.attack_damage, 75);
    assert_eq!(combat.attack_range, 5);
    assert_eq!(combat.attack_cooldown_max, 20);
}

#[test]
fn weaponless_unit_keeps_zero_attack_damage() {
    let doc = IniDocument::parse(
        b"[VehicleTypes]\n0=TRUCKA\n\
[TRUCKA]\nStrength=150\nSpeed=40\nSight=4\nCost=100\nArmor=light\n",
    )
    .unwrap();
    let rules = RulesSystem {
        edition: GameEdition::Ra2,
        rules: doc.clone(),
        art: IniDocument::default(),
        globals: RulesGlobals::from_rules(&doc),
        overlay_types: OverlayTypeRegistry::default(),
        terrain_spawners: TerrainSpawnerDefinitions::default(),
        color_schemes: ColorSchemes::default(),
        countries: CountryRegistry::default(),
        techno_types: TechnoTypeRegistry::from_rules(&doc),
        warheads: WarheadRegistry::default(),
        super_weapons: SuperWeaponTypeRegistry::default(),
    };
    let mut map = MapInfo::empty(GameEdition::Ra2, "civilian");
    map.width = 16;
    map.height = 16;
    map.entities.push(MapEntity {
        kind: MapEntityKind::Unit,
        owner: "Neutral".into(),
        type_id: "TRUCKA".into(),
        health: 256,
        x: 4,
        y: 4,
        facing: 0,
        sub_cell: 0,
        mission: String::new(),
        tag: String::new(),
    });
    let world = battle_from_rules(&rules, map);
    let combat = world.ecs_combat_view(world.entity_id_at(0).expect("entity")).expect("combat");
    assert_eq!(combat.attack_damage, 0, "无主武器时不得用 Strength 发明伤害");
}
