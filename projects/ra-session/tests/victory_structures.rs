//! 胜负计入建筑作战力量。

mod common;

use common::rules_with_mtnk;
use ra_map::{MapEntity, MapEntityKind, MapInfo};
use ra_session::Session;
use ra_types::GameEdition;
use ra_world::World;

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
    let mut session = Session::new(World::new(GameEdition::Ra2, &rules, map), "victory");
    session.world.entities[1].kind = MapEntityKind::Structure;
    session.world.entities[1].type_id = "NACNST".into();
    assert!(session.sole_victor().is_none());
    session.world.entities[1].dead = true;
    assert_eq!(session.sole_victor(), Some("Americans"));
}
