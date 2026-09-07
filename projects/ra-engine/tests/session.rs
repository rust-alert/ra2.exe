//! 会话 tick、选中、快照与联机摘要集成测试。

mod common;

use common::rules_with_mtnk;
use ra_engine::{GameCommand, MAX_TICKS_PER_PUMP, MatchOutcome, Session, World};
use ra_map::{MapEntity, MapEntityKind, MapInfo, Waypoint};
use ra_types::GameEdition;

#[test]
fn session_tick_and_snapshot() {
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
    let world = World::new(GameEdition::Ra2, &rules, map);
    let mut session = Session::new(world, "test");
    session.push_command(GameCommand::MoveTo { entity_index: 0, x: 12, y: 10 });
    session.tick();
    let snap = session.snapshot();
    assert_eq!(snap.tick, 1);
    assert_eq!(snap.units.len(), 1);
    assert_eq!(snap.units[0].x, 11);
}

#[test]
fn selection_orders_attack_and_detects_victor() {
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
    let mut session = Session::new(World::new(GameEdition::Ra2, &rules, map), "t");
    session.world.entities[0].target_x = None;
    session.world.entities[0].target_y = None;
    session.world.entities[1].target_x = None;
    session.world.entities[1].target_y = None;
    session.world.entities[1].speed = 0;
    session.cycle_selection();
    assert_eq!(session.selected, vec![0]);
    session.select_add(1);
    assert_eq!(session.selected, vec![0]); // 异阵营拒绝
    session.select_all_of_owner(0);
    assert_eq!(session.selected, vec![0]);
    let foe = session.nearest_hostile(0).unwrap();
    assert_eq!(foe, 1);
    session.order_selected_attack(foe);
    for _ in 0..80 {
        session.tick();
        if session.sole_victor().is_some() {
            break;
        }
    }
    assert_eq!(session.sole_victor(), Some("Americans"));
    assert!(session.world.entities[1].dead);
    assert_eq!(session.outcome, Some(MatchOutcome::Victory { owner: "Americans".into() }));
    assert!(session.paused);
    assert_eq!(session.pump(1.0), 0);
}

#[test]
fn image_to_cell_uses_preview_origin() {
    let rules = rules_with_mtnk();
    let mut map = MapInfo::empty(GameEdition::Ra2, "t");
    map.width = 20;
    map.height = 30;
    let mut session = Session::new(World::new(GameEdition::Ra2, &rules, map), "t");
    session.set_preview_origin(-100, -50);
    // 钻石中心在等距空间；减去 origin 得到图像坐标。
    let (sx, sy) = ra_map::iso_to_screen(5, 4, 0);
    let cx = (sx + ra_map::TILE_WIDTH / 2) as f32;
    let cy = (sy + ra_map::TILE_HEIGHT / 2) as f32;
    let ix = cx - (-100.0);
    let iy = cy - (-50.0);
    assert_eq!(session.image_to_cell(ix, iy), Some((5, 4)));
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
    let mut session = Session::new(World::new(GameEdition::Ra2, &rules, map), "t");
    session.set_preview_origin(-100, -50);
    session.select_only(0);
    let snap = session.snapshot();
    assert_eq!(snap.selected, vec![0]);
    assert_eq!(snap.units.len(), 1);
    let u = &snap.units[0];
    let z = session.world.pass_grid.cell_height(5, 4);
    let (sx, sy) = ra_map::iso_to_screen(5, 4, z);
    assert_eq!(u.screen_x, sx - (-100));
    assert_eq!(u.screen_y, sy - (-50));
}

#[test]
fn pump_advances_fixed_hz_ticks() {
    let rules = rules_with_mtnk();
    let map = MapInfo::empty(GameEdition::Ra2, "t");
    let mut session = Session::new(World::new(GameEdition::Ra2, &rules, map), "t");
    session.tick_hz = 10;
    assert_eq!(session.pump(0.05), 0); // 50ms < 100ms
    assert_eq!(session.world.tick, 0);
    assert_eq!(session.pump(0.05), 1); // 累计 100ms
    assert_eq!(session.world.tick, 1);
    assert_eq!(session.pump(1.0), MAX_TICKS_PER_PUMP); // 追赶有上限
}

#[test]
fn remote_digest_mismatch_pauses() {
    let rules = rules_with_mtnk();
    let map = MapInfo::empty(GameEdition::Ra2, "t");
    let mut session = Session::new(World::new(GameEdition::Ra2, &rules, map), "t");
    session.tick();
    let mut bad = session.local_digest();
    bad.hash ^= 0xff;
    assert!(!session.apply_remote_digest(&bad));
    assert!(session.paused);
    assert!(session.pause_reason.is_some());
    assert_eq!(session.pump(1.0), 0);
    session.resume();
    assert!(!session.paused);
    assert!(session.pump(0.2) >= 1);
}
