//! nearest_hostile 可指向敌方建筑；同盟不算敌对。

use crate::common::{battle_from_defs, defs_from_rules_ini};
use ra_engine::Session;
use ra_map::{MapEntity, MapEntityKind, MapInfo};
use ra_types::{EntityId, GameEdition};

#[test]
fn nearest_hostile_includes_structures() {
    let defs = defs_from_rules_ini(
        b"[VehicleTypes]\n0=MTNK\n\
[BuildingTypes]\n0=NACNST\n\
[MTNK]\nStrength=400\nSpeed=64\nSight=6\nCost=800\nArmor=heavy\n\
[NACNST]\nStrength=1000\nArmor=wood\nFoundation=2x2\n",
    );
    let mut map = MapInfo::empty(GameEdition::Ra2, "hostile-bldg");
    map.width = 16;
    map.height = 16;
    map.entities.push(MapEntity {
        kind: MapEntityKind::Unit,
        owner: "AMERICANS".into(),
        type_id: "MTNK".into(),
        health: 256,
        x: 4,
        y: 4,
        facing: 0,
        sub_cell: 0,
        mission: Default::default(),
        tag: Default::default(),
    });
    map.entities.push(MapEntity {
        kind: MapEntityKind::Structure,
        owner: "SOVIETS".into(),
        type_id: "MTNK".into(),
        health: 256,
        x: 7,
        y: 4,
        facing: 0,
        sub_cell: 0,
        mission: Default::default(),
        tag: Default::default(),
    });
    let mut session = Session::from_state(battle_from_defs(GameEdition::Ra2, defs.clone(), map), "hostile-bldg");
    let enemy = session.expect_battle().world.entity_id_at(1).expect("entity");
    assert!(session.expect_battle_mut().world.set_ecs_type_id(enemy, "NACNST", MapEntityKind::Structure));
    assert_eq!(session.expect_battle().nearest_hostile(EntityId(1)), Some(EntityId(2)));
}

#[test]
fn nearest_hostile_skips_allied_houses_for_all_editions() {
    // 大厅队伍写入 allies 后，同号同盟不得被索敌为敌对（含 YR）。
    let defs = defs_from_rules_ini(
        b"[VehicleTypes]\n0=MTNK\n\
[MTNK]\nStrength=400\nSpeed=64\nSight=6\nCost=800\nArmor=heavy\n",
    );
    let mut map = MapInfo::empty(GameEdition::Yr, "ally-hostile");
    map.width = 16;
    map.height = 16;
    for (owner, x) in [("AMERICANS", 4u16), ("FRANCE", 5), ("SOVIETS", 10)] {
        map.entities.push(MapEntity {
            kind: MapEntityKind::Unit,
            owner: owner.into(),
            type_id: "MTNK".into(),
            health: 256,
            x,
            y: 4,
            facing: 0,
            sub_cell: 0,
            mission: Default::default(),
            tag: Default::default(),
        });
    }
    let mut session = Session::from_state(battle_from_defs(GameEdition::Yr, defs, map), "ally-hostile");
    session
        .expect_battle_mut()
        .world
        .apply_skirmish_lobby_teams(&["AMERICANS", "FRANCE", "SOVIETS"], &[1, 1, 2]);
    let americans = session.expect_battle().world.entity_id_at(0).expect("americans");
    let france = session.expect_battle().world.entity_id_at(1).expect("france");
    let soviets = session.expect_battle().world.entity_id_at(2).expect("soviets");
    assert_eq!(
        session.expect_battle().nearest_hostile(americans),
        Some(soviets),
        "ally FRANCE must not be preferred over SOVIETS"
    );
    assert_ne!(session.expect_battle().nearest_hostile(americans), Some(france));
}
