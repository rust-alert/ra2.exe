//! 每 tick 记录输入帧（含空帧）。

use crate::common::{battle_from_defs, defs_with_mtnk, map_with_size};
use ra_engine::GameCommand;
use ra_types::{EntityId, GameEdition};

#[test]
fn every_tick_records_input_frame_including_empty() {
    let defs = defs_with_mtnk();
    let map = map_with_size();
    let mut world = battle_from_defs(GameEdition::Ra2, defs.clone(), map);
    world.advance_tick();
    assert_eq!(world.last_input_frame().tick, 1);
    assert!(world.last_input_frame().is_empty());
    world.push_command(GameCommand::MoveTo { entity: EntityId(1), x: 1, y: 1 });
    // 无实体时命令被应用但帧仍记录。
    world.advance_tick();
    assert_eq!(world.last_input_frame().tick, 2);
    assert_eq!(world.last_input_frame().commands.len(), 1);
    let scheduled = &world.last_input_frame().commands[0];
    assert_eq!(scheduled.tick.0, 2);
    assert_eq!(scheduled.body, GameCommand::MoveTo { entity: EntityId(1), x: 1, y: 1 });
}
