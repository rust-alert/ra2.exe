//! nearest_hostile 可指向敌方建筑。

use crate::common::rules_with_mtnk;
use ra_engine::{BattleState, Session};
use ra_map::{MapEntity, MapEntityKind, MapInfo};
use ra_types::{EntityId, GameEdition};

#[test]
fn nearest_hostile_includes_structures() {
    let rules = rules_with_mtnk();
    let mut map = MapInfo::empty(GameEdition::Ra2, "hostile-bldg");
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
        x: 7,
        y: 4,
        facing: 0,
        sub_cell: 0,
        mission: String::new(),
        tag: String::new(),
    });
    let mut session = Session::from_state(BattleState::new(GameEdition::Ra2, &rules, map), "hostile-bldg");
    let enemy = session.expect_battle().world.entity_id_at(1).expect("entity");
    assert!(session.expect_battle_mut().world.set_ecs_type_id(enemy, "NACNST", MapEntityKind::Structure));
    assert_eq!(session.expect_battle().nearest_hostile(EntityId(1)), Some(EntityId(2)));
}
