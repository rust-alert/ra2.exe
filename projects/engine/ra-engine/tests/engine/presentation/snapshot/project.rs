//! `project_units` 按 ID 投影。

use crate::common::duel_mtnk_world;
use ra_engine::Session;
use ra_types::EntityId;

#[test]
fn project_units_returns_only_requested_ids() {
    let world = duel_mtnk_world();
    let session = Session::from_state(world, "project");
    let game = session.expect_game();
    let id0 = game.world.entity_id_at(0).expect("entity");
    let id1 = game.world.entity_id_at(1).expect("entity");
    let units = game.project_units(&[id1, EntityId(999_999), id0]);
    assert_eq!(units.len(), 2);
    assert_eq!(units[0].id, id1);
    assert_eq!(units[1].id, id0);
}
