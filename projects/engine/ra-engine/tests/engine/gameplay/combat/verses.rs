//! 弹头 Verses 相对护甲结算伤害。

use crate::common::{battle_from_defs, defs_from_rules_ini};
use ra_engine::GameCommand;
use ra_map::{MapEntity, MapEntityKind, MapInfo};
use ra_types::{EntityId, GameEdition};

#[test]
fn verses_scales_damage_against_armor() {
    let defs = defs_from_rules_ini(
        b"[VehicleTypes]\n0=ATK\n1=TGT\n\
[ATK]\nStrength=400\nSpeed=64\nSight=6\nCost=800\nPrimary=Gun\nArmor=none\n\
[TGT]\nStrength=400\nSpeed=0\nSight=1\nCost=100\nArmor=heavy\n\
[Gun]\nDamage=100\nROF=1\nRange=8\nWarhead=AP\n\
[AP]\nVerses=100%,100%,100%,100%,100%,50%,100%,100%,100%,100%,100%\n",
    );
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
        tag: Default::default(),
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
        tag: Default::default(),
    });
    let mut world = battle_from_defs(GameEdition::Ra2, defs, map);
    assert_eq!(world.ecs_combat_view(world.entity_id_at(0).expect("entity")).expect("combat").attack_damage, 100);
    assert_eq!(world.ecs_combat_view(world.entity_id_at(0).expect("entity")).expect("combat").attack_verses[5], 50);
    assert_eq!(world.ecs_combat_view(world.entity_id_at(1).expect("entity")).expect("combat").armor, ra_types::ArmorKind::Heavy);
    world.push_command(GameCommand::Attack { attacker: EntityId(1), target: EntityId(2) });
    world.advance_tick();
    // 100 * 50% = 50
    assert_eq!(world.ecs_health(world.entity_id_at(1).expect("entity")).expect("health").0, 350);
}
