//! 弹头 Verses 相对护甲结算伤害。

use ra_adaptor::RulesDb;
use ra_assets::{ColorSchemes, IniDocument, OverlayTypeRegistry, TechnoTypeRegistry, WarheadRegistry};
use ra_map::{MapEntity, MapEntityKind, MapInfo};
use ra_types::GameEdition;
use ra_engine::{GameCommand, World};

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
    let rules = RulesDb {
        edition: GameEdition::Ra2,
        rules: doc.clone(),
        art: IniDocument::default(),
        overlay_types: OverlayTypeRegistry::default(),
        color_schemes: ColorSchemes::default(),
        techno_types,
        warheads,
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
    });
    let mut world = World::new(GameEdition::Ra2, &rules, map);
    assert_eq!(world.entities[0].attack_damage, 100);
    assert_eq!(world.entities[0].attack_verses[5], 50);
    assert_eq!(world.entities[1].armor, "heavy");
    world.push_command(GameCommand::Attack { attacker_index: 0, target_index: 1 });
    world.advance_tick();
    // 100 * 50% = 50
    assert_eq!(world.entities[1].health, 350);
}
