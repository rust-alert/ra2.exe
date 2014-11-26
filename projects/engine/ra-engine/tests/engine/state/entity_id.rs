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
    assert_eq!(world.players[0].house.as_ref(), "Americans");
    assert_eq!(world.players[1].house.as_ref(), "Russians");
    assert_eq!(world.players[0].funds, 0);
}

#[test]
fn registers_ecs_handles_for_seeded_entities() {
    let world = duel_mtnk_world();
    assert_eq!(world.ecs_registry_len(), world.entities.len());
    assert_eq!(world.ecs_alive_count(), world.entities.len());
    assert!(world.has_ecs_entity(EntityId(1)));
    assert!(world.has_ecs_entity(EntityId(2)));
    assert!(!world.has_ecs_entity(EntityId(99)));
}
