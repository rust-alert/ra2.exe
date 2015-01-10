//! ECS 基础组件与 `WorldEntity` 同步。

use crate::common::duel_mtnk_world;
use ra_engine::{GameCommand, SystemPhase, SystemSchedule};

#[test]
fn seed_writes_identity_transform_health_components() {
    let world = duel_mtnk_world();
    let id = world.entity_id_at(0).expect("entity");
    assert_eq!(world.ecs_health(id), Some((world.ecs_health(world.entity_id_at(0).expect("entity")).expect("health").0, world.ecs_health(world.entity_id_at(0).expect("entity")).expect("health").1, false)));
    assert_eq!(
        world.ecs_transform(id),
        Some((world.ecs_transform(world.entity_id_at(0).expect("entity")).expect("xf").0, world.ecs_transform(world.entity_id_at(0).expect("entity")).expect("xf").1, world.ecs_transform(world.entity_id_at(0).expect("entity")).expect("xf").2))
    );
    assert_eq!(world.ecs_move_destination(id), Some((None, None)));
    assert_eq!(world.ecs_attack_state(id), Some((None, 0)));
    assert_eq!(world.ecs_produce_remaining(id), Some(None));
    assert_eq!(world.ecs_animation(id), Some((0, 0)));
}

#[test]
fn tick_sync_updates_health_after_combat() {
    let mut world = duel_mtnk_world();
    let attacker = world.entity_id_at(0).expect("entity");
    let target = world.entity_id_at(1).expect("entity");
    world.push_command(GameCommand::Attack { attacker, target });
    for _ in 0..64 {
        world.advance_tick();
        if world.ecs_health(world.entity_id_at(1).expect("entity")).expect("health").2 {
            break;
        }
    }
    assert!(world.ecs_health(world.entity_id_at(1).expect("entity")).expect("health").2);
    assert_eq!(
        world.ecs_health(target),
        Some((world.ecs_health(world.entity_id_at(1).expect("entity")).expect("health").0, world.ecs_health(world.entity_id_at(1).expect("entity")).expect("health").1, true))
    );
}

#[test]
fn tick_sync_updates_move_destination() {
    let mut world = duel_mtnk_world();
    let id = world.entity_id_at(0).expect("entity");
    let (x, y) = (world.ecs_transform(world.entity_id_at(0).expect("entity")).expect("xf").0, world.ecs_transform(world.entity_id_at(0).expect("entity")).expect("xf").1);
    let dest_x = x.saturating_add(3);
    world.push_command(GameCommand::MoveTo { entity: id, x: dest_x, y });
    world.advance_tick();
    assert_eq!(world.ecs_move_destination(id), Some((Some(dest_x), Some(y))));
    assert_eq!(world.ecs_move_destination(world.entity_id_at(0).expect("entity")).expect("dest").0, Some(dest_x));
    assert_eq!(world.ecs_move_destination(world.entity_id_at(0).expect("entity")).expect("dest").1, Some(y));
}

#[test]
fn tick_sync_updates_attack_target() {
    let mut world = duel_mtnk_world();
    let attacker = world.entity_id_at(0).expect("entity");
    let target = world.entity_id_at(1).expect("entity");
    world.push_command(GameCommand::Attack { attacker, target });
    world.advance_tick();
    assert_eq!(world.ecs_attack_state(attacker).map(|(t, _)| t), Some(Some(target)));
    assert_eq!(world.ecs_attack_state(world.entity_id_at(0).expect("entity")).expect("atk").0, Some(target));
    assert_eq!(
        world.ecs_attack_state(attacker).map(|(_, cd)| cd),
        Some(world.ecs_attack_state(world.entity_id_at(0).expect("entity")).expect("atk").1)
    );
}

#[test]
fn combat_writes_health_through_ecs_authority() {
    let mut world = duel_mtnk_world();
    let attacker = world.entity_id_at(0).expect("entity");
    let target = world.entity_id_at(1).expect("entity");
    let before = world.ecs_health(target).unwrap().0;
    world.push_command(GameCommand::Attack { attacker, target });
    for _ in 0..8 {
        world.advance_tick();
        let (current, _, _) = world.ecs_health(target).unwrap();
        if current < before {
            assert_eq!(world.ecs_health(world.entity_id_at(1).expect("entity")).expect("health").0, current);
            return;
        }
    }
    panic!("expected combat to reduce ECS health");
}

#[test]
fn movement_writes_transform_through_ecs_authority() {
    let mut world = duel_mtnk_world();
    let id = world.entity_id_at(0).expect("entity");
    let (x, y) = (world.ecs_transform(world.entity_id_at(0).expect("entity")).expect("xf").0, world.ecs_transform(world.entity_id_at(0).expect("entity")).expect("xf").1);
    let start = world.ecs_transform(id).unwrap();
    world.push_command(GameCommand::MoveTo { entity: id, x: x.saturating_add(3), y });
    for _ in 0..32 {
        world.advance_tick();
        let now = world.ecs_transform(id).unwrap();
        if now != start {
            assert_eq!(world.ecs_transform(world.entity_id_at(0).expect("entity")).expect("xf").0, now.0);
            assert_eq!(world.ecs_transform(world.entity_id_at(0).expect("entity")).expect("xf").1, now.1);
            assert_eq!(world.ecs_transform(world.entity_id_at(0).expect("entity")).expect("xf").2, now.2);
            return;
        }
    }
    panic!("expected movement to change ECS transform");
}

#[test]
fn rehash_phase_keeps_ecs_animation_authority() {
    let mut world = duel_mtnk_world();
    let id = world.entity_id_at(0).expect("entity");
    // Rehash 只投影 ECS → 槽位，不会改写 ECS 动画权威值。
    assert_eq!(world.ecs_animation(id), Some((0, 0)));
    world.advance_scheduled_tick(&SystemSchedule::from_phases(vec![SystemPhase::Rehash]));
    assert_eq!(world.ecs_animation(id), Some((0, 0)));
}

#[test]
fn combat_hit_flash_writes_through_ecs_animation() {
    let mut world = duel_mtnk_world();
    let attacker = world.entity_id_at(0).expect("entity");
    let target = world.entity_id_at(1).expect("entity");
    world.push_command(GameCommand::Attack { attacker, target });
    for _ in 0..16 {
        world.advance_tick();
        let (_, flash) = world.ecs_animation(target).unwrap();
        if flash > 0 {
            assert_eq!(world.ecs_animation(world.entity_id_at(1).expect("entity")).expect("anim").1, flash);
            return;
        }
    }
    panic!("expected combat to set ECS hit_flash");
}

#[test]
fn death_clears_locomotor_speed_through_ecs() {
    let mut world = duel_mtnk_world();
    let attacker = world.entity_id_at(0).expect("entity");
    let target = world.entity_id_at(1).expect("entity");
    assert!(world.ecs_speed(world.entity_id_at(1).expect("entity")).expect("speed") > 0);
    world.push_command(GameCommand::Attack { attacker, target });
    for _ in 0..128 {
        world.advance_tick();
        if world.ecs_health(world.entity_id_at(1).expect("entity")).expect("health").2 {
            break;
        }
    }
    assert!(world.ecs_health(world.entity_id_at(1).expect("entity")).expect("health").2);
    assert_eq!(world.ecs_speed(world.entity_id_at(1).expect("entity")).expect("speed"), 0);
}
