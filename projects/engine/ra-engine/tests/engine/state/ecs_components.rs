//! ECS 基础组件与 `WorldEntity` 同步。

use std::sync::Arc;

use crate::common::duel_mtnk_world;
use ra_engine::{GameCommand, SystemPhase, SystemSchedule};

#[test]
fn seed_writes_identity_transform_health_components() {
    let world = duel_mtnk_world();
    let id = world.entities[0].id;
    assert_eq!(world.ecs_health(id), Some((world.entities[0].health, world.entities[0].max_health, false)));
    assert_eq!(
        world.ecs_transform(id),
        Some((world.entities[0].x, world.entities[0].y, world.entities[0].facing))
    );
    assert_eq!(world.ecs_move_destination(id), Some((None, None)));
    assert_eq!(world.ecs_attack_state(id), Some((None, 0)));
    assert_eq!(world.ecs_produce_remaining(id), Some(None));
    assert_eq!(world.ecs_animation(id), Some((0, 0)));
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

#[test]
fn tick_sync_updates_move_destination() {
    let mut world = duel_mtnk_world();
    let id = world.entities[0].id;
    let (x, y) = (world.entities[0].x, world.entities[0].y);
    let dest_x = x.saturating_add(3);
    world.push_command(GameCommand::MoveTo { entity: id, x: dest_x, y });
    world.advance_tick();
    assert_eq!(world.ecs_move_destination(id), Some((Some(dest_x), Some(y))));
    assert_eq!(world.entities[0].target_x, Some(dest_x));
    assert_eq!(world.entities[0].target_y, Some(y));
}

#[test]
fn tick_sync_updates_attack_target() {
    let mut world = duel_mtnk_world();
    let attacker = world.entities[0].id;
    let target = world.entities[1].id;
    world.push_command(GameCommand::Attack { attacker, target });
    world.advance_tick();
    assert_eq!(world.ecs_attack_state(attacker).map(|(t, _)| t), Some(Some(target)));
    assert_eq!(world.entities[0].attack_target, Some(target));
    assert_eq!(
        world.ecs_attack_state(attacker).map(|(_, cd)| cd),
        Some(world.entities[0].attack_cooldown)
    );
}

#[test]
fn combat_writes_health_through_ecs_authority() {
    let mut world = duel_mtnk_world();
    let attacker = world.entities[0].id;
    let target = world.entities[1].id;
    let before = world.ecs_health(target).unwrap().0;
    world.push_command(GameCommand::Attack { attacker, target });
    for _ in 0..8 {
        world.advance_tick();
        let (current, _, _) = world.ecs_health(target).unwrap();
        if current < before {
            assert_eq!(world.entities[1].health, current);
            return;
        }
    }
    panic!("expected combat to reduce ECS health");
}

#[test]
fn movement_writes_transform_through_ecs_authority() {
    let mut world = duel_mtnk_world();
    let id = world.entities[0].id;
    let (x, y) = (world.entities[0].x, world.entities[0].y);
    let start = world.ecs_transform(id).unwrap();
    world.push_command(GameCommand::MoveTo { entity: id, x: x.saturating_add(3), y });
    for _ in 0..32 {
        world.advance_tick();
        let now = world.ecs_transform(id).unwrap();
        if now != start {
            assert_eq!(world.entities[0].x, now.0);
            assert_eq!(world.entities[0].y, now.1);
            assert_eq!(world.entities[0].facing, now.2);
            return;
        }
    }
    panic!("expected movement to change ECS transform");
}

#[test]
fn rehash_phase_syncs_production_and_animation() {
    let mut world = duel_mtnk_world();
    let id = world.entities[0].id;
    world.entities[0].produce_queue = Some((Arc::from("E1"), 7));
    world.entities[0].rally_x = Some(11);
    world.entities[0].rally_y = Some(12);
    world.entities[0].hva_frame = 3;
    world.entities[0].hit_flash = 2;
    world.advance_scheduled_tick(&SystemSchedule::from_phases(vec![SystemPhase::Rehash]));
    assert_eq!(world.ecs_produce_remaining(id), Some(Some(7)));
    assert_eq!(world.ecs_animation(id), Some((3, 2)));
}
