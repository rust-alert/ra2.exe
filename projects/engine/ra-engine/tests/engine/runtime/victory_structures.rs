//! 胜负计入建筑作战力量。

use crate::common::{defs_with_mtnk, battle_from_defs};
use ra_engine::Session;
use ra_map::{MapEntity, MapEntityKind, MapInfo};
use ra_types::GameEdition;

#[test]
fn living_structure_prevents_sole_victor() {
    let defs = defs_with_mtnk();
    let mut map = MapInfo::empty(GameEdition::Ra2, "victory");
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
        type_id: "MTNK".into(),
        health: 256,
        x: 8,
        y: 8,
        facing: 0,
        sub_cell: 0,
        mission: String::new(),
        tag: String::new(),
    });
    // 结构体借用 MTNK 规则仅作 Strength；种类为 Structure 即计入作战力量。
    let mut session = Session::from_state(battle_from_defs(GameEdition::Ra2, defs.clone(), map), "victory");
    let enemy = session.expect_battle().world.entity_id_at(1).expect("entity");
    let max = session.expect_battle().world.ecs_health(session.expect_battle().world.entity_id_at(1).expect("entity")).expect("health").1;
    assert!(session.expect_battle_mut().world.set_ecs_type_id(enemy, "NACNST", MapEntityKind::Structure));
    assert_eq!(session.expect_battle_mut().world.players.len(), 2);
    assert!(session.expect_battle().sole_victor().is_none());
    assert!(session.expect_battle_mut().world.set_ecs_health(enemy, 0, max, true));
    assert_eq!(session.expect_battle().sole_victor(), Some("AMERICANS"));
}

#[test]
fn ambient_units_do_not_block_sole_victor() {
    let defs = defs_with_mtnk();
    let mut map = MapInfo::empty(GameEdition::Ra2, "ambient-victory");
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
        kind: MapEntityKind::Unit,
        owner: "Soviets".into(),
        type_id: "MTNK".into(),
        health: 256,
        x: 8,
        y: 4,
        facing: 0,
        sub_cell: 0,
        mission: String::new(),
        tag: String::new(),
    });
    map.entities.push(MapEntity {
        kind: MapEntityKind::Unit,
        owner: "Neutral".into(),
        type_id: "MTNK".into(),
        health: 256,
        x: 12,
        y: 4,
        facing: 0,
        sub_cell: 0,
        mission: String::new(),
        tag: String::new(),
    });
    let mut session = Session::from_state(battle_from_defs(GameEdition::Ra2, defs.clone(), map), "ambient-victory");
    assert_eq!(session.expect_battle().world.players.len(), 3);
    assert!(session.expect_battle().sole_victor().is_none());
    let enemy = session.expect_battle().world.entity_id_at(1).expect("entity");
    let max = session.expect_battle().world.ecs_health(enemy).expect("health").1;
    assert!(session.expect_battle_mut().world.set_ecs_health(enemy, 0, max, true));
    assert_eq!(session.expect_battle().sole_victor(), Some("AMERICANS"));
}
