//! `EntityId` 与玩家状态播种。

use crate::common::duel_mtnk_world;
use ra_types::EntityId;

#[test]
fn seeds_stable_entity_ids_and_players() {
    let world = duel_mtnk_world();
    assert_eq!(world.entities[0].id, EntityId(1));
    assert_eq!(world.entities[1].id, EntityId(2));
    assert_eq!(world.entity_index(EntityId(2)), Some(1));
    assert_eq!(world.entity_index(EntityId(99)), None);
    assert_eq!(world.players.len(), 2);
    assert_eq!(world.players[0].house, "Americans");
    assert_eq!(world.players[1].house, "Russians");
    assert_eq!(world.players[0].funds, 0);
}
