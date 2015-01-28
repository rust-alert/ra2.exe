//! 主武器 Damage / Range 播种到世界实体。

use ra_adaptor::RulesDb;
use ra_assets::{ColorSchemes, IniDocument, OverlayTypeRegistry, TechnoTypeRegistry, WarheadRegistry};
use ra_engine::BattleState;
use ra_map::{MapEntity, MapEntityKind, MapInfo};
use ra_types::GameEdition;

#[test]
fn seeds_attack_stats_from_primary_weapon() {
    let doc = IniDocument::parse(
        b"[VehicleTypes]\n0=MTNK\n\
[MTNK]\nStrength=400\nSpeed=64\nSight=6\nCost=800\nPrimary=90mm\n\
[90mm]\nDamage=75\nROF=20\nRange=5\n",
    )
    .unwrap();
    let rules = RulesDb {
        edition: GameEdition::Ra2,
        rules: doc.clone(),
        art: IniDocument::default(),
        overlay_types: OverlayTypeRegistry::default(),
        color_schemes: ColorSchemes::default(),
        techno_types: TechnoTypeRegistry::from_rules(&doc),
        warheads: WarheadRegistry::default(),
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
    });
    let world = BattleState::new(GameEdition::Ra2, &rules, map);
    let combat = world.ecs_combat_view(world.entity_id_at(0).expect("entity")).expect("combat");
    assert_eq!(combat.attack_damage, 75);
    assert_eq!(combat.attack_range, 5);
    assert_eq!(combat.attack_cooldown_max, 20);
}
