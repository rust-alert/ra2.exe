//! 同一 `CommandId` 不得被执行两次。

use crate::common::duel_mtnk_world;
use ra_engine::{CommandRejectReason, GameCommand};
use ra_types::{CommandId, EntityId, PlayerId, ScheduledCommand, Tick};

#[test]
fn duplicate_command_id_is_rejected_on_second_apply() {
    let mut world = duel_mtnk_world();
    let body = GameCommand::MoveTo { entity: EntityId(1), x: 1, y: 1 };
    let first = ScheduledCommand::new(CommandId(42), PlayerId(0), Tick(1), body.clone());
    let dup = ScheduledCommand::new(CommandId(42), PlayerId(0), Tick(2), body);
    world.push_scheduled(first);
    world.advance_tick();
    assert!(world.last_rejects().is_empty());
    assert_eq!(world.ecs_move_destination(EntityId(1)), Some((Some(1), Some(1))));
    assert_eq!(world.entities[0].target_x, Some(1));

    world.push_scheduled(dup);
    world.advance_tick();
    assert_eq!(world.last_rejects().len(), 1);
    assert_eq!(world.last_rejects()[0].reason, CommandRejectReason::DuplicateCommand);
    // 重复命令被拒绝后，ECS 权威目的地应保持首次执行结果。
    assert_eq!(world.ecs_move_destination(EntityId(1)), Some((Some(1), Some(1))));
    assert_eq!(world.entities[0].target_x, Some(1));
}
