//! 快照包含建筑实体。

use crate::common::{defs_with_mtnk, battle_from_defs};
use ra_engine::Session;
use ra_map::{MapEntity, MapEntityKind, MapInfo};
use ra_types::GameEdition;

#[test]
fn snapshot_includes_structures() {
    let defs = defs_with_mtnk();
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
        mission: String::new(),
        tag: Default::default(),
    });
    let session = Session::from_state(battle_from_defs(GameEdition::Ra2, defs.clone(), map), "struct");
    let snap = session.expect_battle().snapshot(&[]);
    assert_eq!(snap.units.len(), 1);
    assert_eq!(snap.units[0].kind, MapEntityKind::Structure);
    assert_eq!(snap.units[0].type_id.as_ref(), "GACNST");
}
