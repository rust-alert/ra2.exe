//! 快照包含建筑实体。

use crate::common::rules_with_mtnk;
use ra_engine::{Session, World};
use ra_map::{MapEntity, MapEntityKind, MapInfo};
use ra_types::GameEdition;

#[test]
fn snapshot_includes_structures() {
    let rules = rules_with_mtnk();
    let mut map = MapInfo::empty(GameEdition::Ra2, "struct");
    map.width = 12;
    map.height = 12;
    map.entities.push(MapEntity {
        kind: MapEntityKind::Structure,
        owner: "Americans".into(),
        type_id: "GACNST".into(),
        health: 256,
        x: 3,
        y: 3,
        facing: 0,
        sub_cell: 0,
    });
    let session = Session::new(World::new(GameEdition::Ra2, &rules, map), "struct");
    let snap = session.snapshot();
    assert_eq!(snap.units.len(), 1);
    assert_eq!(snap.units[0].kind, MapEntityKind::Structure);
    assert_eq!(snap.units[0].type_id, "GACNST");
}
