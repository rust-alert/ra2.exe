//! 会话 tick、命令、快照与联机摘要集成测试。

use crate::common::{rules_with_mtnk, test_engine};
use ra_engine::{GameCommand, MAX_TICKS_PER_PUMP, BattleOutcome, BattleState, Session};
use ra_map::{MapEntity, MapEntityKind, MapInfo, Waypoint};
use ra_types::{EntityId, GameEdition};

#[test]
fn session_tick_and_snapshot() {
    let engine = test_engine();
    let rules = rules_with_mtnk();
    let mut map = MapInfo::empty(GameEdition::Ra2, "t");
    map.width = 20;
    map.height = 30;
    map.waypoints.push(Waypoint { index: 0, x: 12, y: 10 });
    map.entities.push(MapEntity {
        kind: MapEntityKind::Unit,
        owner: "Americans".into(),
        type_id: "MTNK".into(),
        health: 256,
        x: 10,
        y: 10,
        facing: 0,
        sub_cell: 0,
    });
    let world = BattleState::new(GameEdition::Ra2, &rules, map);
    let mut session = Session::from_state(world, "test");
    session.expect_game_mut().push_command(GameCommand::MoveTo { entity: EntityId(1), x: 12, y: 10 });
    session.tick(&engine.runtime());
    let snap = session.expect_game().snapshot(&[]);
    assert_eq!(snap.tick, 1);
    assert_eq!(snap.units.len(), 1);
    assert_eq!(snap.units[0].x, 11);
}

#[test]
fn order_attack_and_detects_victor() {
    let engine = test_engine();
    let rules = rules_with_mtnk();
    let mut map = MapInfo::empty(GameEdition::Ra2, "t");
    map.width = 20;
    map.height = 30;
    map.entities.push(MapEntity {
        kind: MapEntityKind::Unit,
        owner: "Americans".into(),
        type_id: "MTNK".into(),
        health: 256,
        x: 10,
        y: 10,
        facing: 0,
        sub_cell: 0,
    });
    map.entities.push(MapEntity {
        kind: MapEntityKind::Unit,
        owner: "Russians".into(),
        type_id: "MTNK".into(),
        health: 256,
        x: 12,
        y: 10,
        facing: 0,
        sub_cell: 0,
    });
    let mut session = Session::from_state(BattleState::new(GameEdition::Ra2, &rules, map), "t");
    {
        let world = &mut session.expect_game_mut().world;
        let a = world.entity_id_at(0).expect("entity");
        let b = world.entity_id_at(1).expect("entity");
        assert!(world.clear_ecs_movement(a));
        assert!(world.clear_ecs_movement(b));
        assert!(world.set_ecs_speed(b, 0));
    }
    let foe = session.expect_game().nearest_hostile(EntityId(1)).unwrap();
    assert_eq!(foe, EntityId(2));
    session.expect_game_mut().order_attack(&[EntityId(1)], foe);
    for _ in 0..80 {
        session.tick(&engine.runtime());
        if session.expect_game().sole_victor().is_some() {
            break;
        }
    }
    assert_eq!(session.expect_game().sole_victor(), Some("Americans"));
    assert!(session.expect_game().world.ecs_health(session.expect_game().world.entity_id_at(1).expect("entity")).expect("health").2);
    assert_eq!(session.expect_game().outcome, Some(BattleOutcome::Victory { owner: "Americans".into() }));
    assert!(session.expect_game().paused);
    assert_eq!(session.pump(&engine.runtime(), 1.0), 0);
}

#[test]
fn image_to_cell_uses_preview_origin() {
    let rules = rules_with_mtnk();
    let mut map = MapInfo::empty(GameEdition::Ra2, "t");
    map.width = 20;
    map.height = 30;
    let mut session = Session::from_state(BattleState::new(GameEdition::Ra2, &rules, map), "t");
    session.expect_game_mut().set_preview_origin(-100, -50);
    let (sx, sy) = ra_map::iso_to_screen(5, 4, 0);
    let cx = (sx + ra_map::TILE_WIDTH / 2) as f32;
    let cy = (sy + ra_map::TILE_HEIGHT / 2) as f32;
    let ix = cx - (-100.0);
    let iy = cy - (-50.0);
    assert_eq!(session.expect_game().image_to_cell(ix, iy), Some((5, 4)));
}

#[test]
fn snapshot_includes_screen_coords_and_selection() {
    let rules = rules_with_mtnk();
    let mut map = MapInfo::empty(GameEdition::Ra2, "t");
    map.width = 20;
    map.height = 30;
    map.entities.push(MapEntity {
        kind: MapEntityKind::Unit,
        owner: "Americans".into(),
        type_id: "MTNK".into(),
        health: 256,
        x: 5,
        y: 4,
        facing: 0,
        sub_cell: 0,
    });
    let mut session = Session::from_state(BattleState::new(GameEdition::Ra2, &rules, map), "t");
    session.expect_game_mut().set_preview_origin(-100, -50);
    let snap = session.expect_game().snapshot(&[EntityId(1)]);
    assert_eq!(snap.selected, vec![EntityId(1)]);
    assert_eq!(snap.units.len(), 1);
    let u = &snap.units[0];
    assert_eq!(u.id, EntityId(1));
    let z = session.expect_game().world.pass_grid.cell_height(5, 4);
    let (sx, sy) = ra_map::iso_to_screen(5, 4, z);
    assert_eq!(u.screen_x, sx - (-100));
    assert_eq!(u.screen_y, sy - (-50));
}

#[test]
fn pump_advances_fixed_hz_ticks() {
    let engine = test_engine();
    let rules = rules_with_mtnk();
    let map = MapInfo::empty(GameEdition::Ra2, "t");
    let mut session = Session::from_state(BattleState::new(GameEdition::Ra2, &rules, map), "t");
    session.tick_hz = 10;
    assert_eq!(session.pump(&engine.runtime(), 0.05), 0);
    assert_eq!(session.expect_game().world.tick, 0);
    assert_eq!(session.pump(&engine.runtime(), 0.05), 1);
    assert_eq!(session.expect_game().world.tick, 1);
    assert_eq!(session.pump(&engine.runtime(), 1.0), MAX_TICKS_PER_PUMP);
}

#[test]
fn remote_digest_mismatch_pauses() {
    let engine = test_engine();
    let rules = rules_with_mtnk();
    let map = MapInfo::empty(GameEdition::Ra2, "t");
    let mut session = Session::from_state(BattleState::new(GameEdition::Ra2, &rules, map), "t");
    session.tick(&engine.runtime());
    let mut bad = session.expect_game().local_digest();
    bad.hash ^= 0xff;
    assert!(!session.expect_game_mut().apply_remote_digest(&bad));
    assert!(session.expect_game().paused);
    assert!(session.expect_game().pause_reason.is_some());
    assert_eq!(session.pump(&engine.runtime(), 1.0), 0);
    session.expect_game_mut().resume();
    assert!(!session.expect_game().paused);
    assert!(session.pump(&engine.runtime(), 0.2) >= 1);
}

#[test]
fn remote_digest_tick_mismatch_is_not_success() {
    let rules = rules_with_mtnk();
    let map = MapInfo::empty(GameEdition::Ra2, "t");
    let mut session = Session::from_state(BattleState::new(GameEdition::Ra2, &rules, map), "t");
    let local = session.expect_game().local_digest();
    let remote = ra_net::StateDigest { tick: local.tick.wrapping_add(1), hash: local.hash };
    assert!(!session.expect_game_mut().apply_remote_digest(&remote));
    assert!(!session.expect_game().paused, "tick 不一致不应当成哈希冲突暂停");
}
