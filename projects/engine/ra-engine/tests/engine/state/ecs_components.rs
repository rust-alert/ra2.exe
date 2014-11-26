//! ECS 基础组件与 `WorldEntity` 同步。

use crate::common::duel_mtnk_world;
use ra_engine::GameCommand;

#[test]
fn seed_writes_identity_transform_health_components() {
    let world = duel_mtnk_world();
    let id = world.entities[0].id;
    assert_eq!(world.ecs_health(id), Some((world.entities[0].health, world.entities[0].max_health, false)));
    assert_eq!(
        world.ecs_transform(id),
        Some((world.entities[0].x, world.entities[0].y, world.entities[0].facing))
    );
}

#[test]
fn tick_sync_updates_health_after_combat() {
    let mut world = duel_mtnk_world();
    let attacker = world.entities[0].id;
    let target = world.entities[1].id;
    world.push_command(GameCommand::Attack { attacker, target });
    for _ in 0..64 {
        world.advance_tick();
        if world.entities[1].dead {
            break;
        }
    }
    assert!(world.entities[1].dead);
    assert_eq!(
        world.ecs_health(target),
        Some((world.entities[1].health, world.entities[1].max_health, true))
    );
}
