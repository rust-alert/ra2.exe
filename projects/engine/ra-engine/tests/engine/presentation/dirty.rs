//! 呈现脏实体集。

use crate::common::duel_mtnk_world;
use ra_engine::DirtyEntitySet;
use ra_types::EntityId;

#[test]
fn dirty_set_drains_sorted_unique() {
    let mut dirty = DirtyEntitySet::new();
    dirty.mark(EntityId(3));
    dirty.mark(EntityId(1));
    dirty.mark(EntityId(3));
    dirty.mark(EntityId(2));
    assert_eq!(dirty.drain(), vec![EntityId(1), EntityId(2), EntityId(3)]);
    assert!(dirty.is_empty());
}

#[test]
fn world_seed_marks_all_entities_dirty() {
    let mut world = duel_mtnk_world();
    let drained = world.take_presentation_dirty();
    assert_eq!(drained.len(), world.entity_count());
    assert!(world.presentation_dirty().is_empty());
}
