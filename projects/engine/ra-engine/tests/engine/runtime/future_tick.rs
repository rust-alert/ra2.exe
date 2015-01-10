//! 调度信封按目标 tick 执行，未来命令不得提前生效。

use crate::common::duel_mtnk_world;
use ra_engine::GameCommand;
use ra_types::{CommandId, EntityId, PlayerId, ScheduledCommand, Tick};

#[test]
fn future_scheduled_command_does_not_apply_early() {
    let mut world = duel_mtnk_world();
    let start_tick = world.tick;
    let future = Tick(start_tick.wrapping_add(3));
    world.push_scheduled(ScheduledCommand::new(CommandId(9001), PlayerId(0), future, GameCommand::MoveTo { entity: EntityId(1), x: 1, y: 1 }));

    world.advance_tick();
    assert_eq!(world.tick, start_tick.wrapping_add(1));
    assert!(world.last_input_frame().commands.is_empty());
    assert_ne!(world.ecs_move_destination(world.entity_id_at(0).expect("entity")).expect("dest").0, Some(1));

    world.advance_tick();
    assert!(world.last_input_frame().commands.is_empty());
    assert_ne!(world.ecs_move_destination(world.entity_id_at(0).expect("entity")).expect("dest").0, Some(1));

    world.advance_tick();
    assert_eq!(world.tick, future.0);
    assert_eq!(world.last_input_frame().commands.len(), 1);
    assert_eq!(world.ecs_move_destination(world.entity_id_at(0).expect("entity")).expect("dest").0, Some(1));
}
