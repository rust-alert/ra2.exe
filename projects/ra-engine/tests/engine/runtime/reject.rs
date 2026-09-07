//! 命令拒绝记录。

use crate::common::duel_mtnk_world;
use ra_engine::{CommandRejectReason, GameCommand};
use ra_types::EntityId;

#[test]
fn records_reject_for_missing_entity_command() {
    let mut world = duel_mtnk_world();
    world.push_command(GameCommand::MoveTo { entity: EntityId(99), x: 1, y: 1 });
    world.advance_tick();
    assert_eq!(world.last_rejects().len(), 1);
    assert_eq!(world.last_rejects()[0].reason, CommandRejectReason::EntityNotFound);
    assert_eq!(world.entities[0].x, 4);
}

#[test]
fn records_reject_for_self_attack() {
    let mut world = duel_mtnk_world();
    world.push_command(GameCommand::Attack { attacker: EntityId(1), target: EntityId(1) });
    world.advance_tick();
    assert_eq!(world.last_rejects()[0].reason, CommandRejectReason::InvalidTarget);
    assert!(world.entities[0].attack_target.is_none());
}
