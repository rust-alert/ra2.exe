//! 主武器 Damage / Range 播种到世界实体。

use crate::common::{battle_from_defs, defs_from_rules_ini};
use ra_map::{MapEntity, MapEntityKind, MapInfo};
use ra_types::GameEdition;

#[test]
fn seeds_attack_stats_from_primary_weapon() {
    let defs = defs_from_rules_ini(
        b"[VehicleTypes]\n0=MTNK\n\
[MTNK]\nStrength=400\nSpeed=64\nSight=6\nCost=800\nPrimary=90mm\n\
[90mm]\nDamage=75\nROF=20\nRange=5\n",
    );
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
    let world = battle_from_defs(GameEdition::Ra2, defs, map);
    let combat = world.ecs_combat_view(world.entity_id_at(0).expect("entity")).expect("combat");
    assert_eq!(combat.attack_damage, 75);
    assert_eq!(combat.attack_range, 5);
    assert_eq!(combat.attack_cooldown_max, 20);
}

#[test]
fn weaponless_unit_keeps_zero_attack_damage() {
    let defs = defs_from_rules_ini(
        b"[VehicleTypes]\n0=TRUCKA\n\
[TRUCKA]\nStrength=150\nSpeed=40\nSight=4\nCost=100\nArmor=light\n",
    );
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
    let world = battle_from_defs(GameEdition::Ra2, defs, map);
    let combat = world.ecs_combat_view(world.entity_id_at(0).expect("entity")).expect("combat");
    assert_eq!(combat.attack_damage, 0, "无主武器时不得用 Strength 发明伤害");
}
