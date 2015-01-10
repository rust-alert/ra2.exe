//! 胜负计入建筑作战力量。

use crate::common::rules_with_mtnk;
use ra_engine::{MatchState, Session};
use ra_map::{MapEntity, MapEntityKind, MapInfo};
use ra_types::GameEdition;

#[test]
fn living_structure_prevents_sole_victor() {
    let rules = rules_with_mtnk();
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
    });
    // 结构体借用 MTNK 规则仅作 Strength；种类为 Structure 即计入作战力量。
    let mut session = Session::from_state(MatchState::new(GameEdition::Ra2, &rules, map), "victory");
    let enemy = session.expect_game().world.entity_id_at(1).expect("entity");
    let max = session.expect_game().world.ecs_health(session.expect_game().world.entity_id_at(1).expect("entity")).expect("health").1;
    assert!(session.expect_game_mut().world.set_ecs_type_id(enemy, "NACNST", MapEntityKind::Structure));
    assert_eq!(session.expect_game_mut().world.players.len(), 2);
    assert!(session.expect_game().sole_victor().is_none());
    assert!(session.expect_game_mut().world.set_ecs_health(enemy, 0, max, true));
    assert_eq!(session.expect_game().sole_victor(), Some("Americans"));
}
