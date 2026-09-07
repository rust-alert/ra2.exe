//! `World` 仿真集成测试（仅通过公开 API）。

use ra_adaptor::RulesDb;
use ra_assets::{ColorSchemes, IniDocument, OverlayTypeRegistry, TechnoKind, TechnoTypeRegistry, WarheadRegistry};
use ra_map::{MapEntity, MapEntityKind, MapInfo, Waypoint};
use ra_types::GameEdition;
use ra_world::{ATTACK_COOLDOWN_TICKS, GameCommand, World};

fn rules_with_mtnk() -> RulesDb {
    let doc = IniDocument::parse(
        b"[VehicleTypes]\n0=MTNK\n\
[MTNK]\nStrength=400\nSpeed=64\nSight=6\nCost=800\nArmor=heavy\n",
    )
    .unwrap();
    RulesDb {
        edition: GameEdition::Ra2,
        rules: doc.clone(),
        art: IniDocument::default(),
        overlay_types: OverlayTypeRegistry::default(),
        color_schemes: ColorSchemes::default(),
        techno_types: TechnoTypeRegistry::from_rules(&doc),
        warheads: WarheadRegistry::default(),
    }
}

fn map_with_size() -> MapInfo {
    let mut map = MapInfo::empty(GameEdition::Ra2, "t");
    map.width = 20;
    map.height = 30;
    map
}

#[test]
fn binds_strength_and_speed() {
    let rules = rules_with_mtnk();
    let mut map = map_with_size();
    map.entities.push(MapEntity {
        kind: MapEntityKind::Unit,
        owner: "Americans".into(),
        type_id: "MTNK".into(),
        health: 256,
        x: 10,
        y: 20,
        facing: 64,
        sub_cell: 0,
    });
    let world = World::new(GameEdition::Ra2, &rules, map);
    assert_eq!(world.entities.len(), 1);
    let e = &world.entities[0];
    assert_eq!(e.max_health, 400);
    assert_eq!(e.health, 400);
    assert_eq!(e.speed, 64);
    assert_eq!(e.attack_range, 6);
    assert_eq!(e.attack_damage, 100);
    assert_eq!(e.attack_cooldown_max, ATTACK_COOLDOWN_TICKS);
    assert_eq!(e.techno_kind, Some(TechnoKind::Vehicle));
    assert_eq!(world.bound_techno_count(), 1);
}

#[test]
fn advances_when_ordered_to_move() {
    let rules = rules_with_mtnk();
    let mut map = map_with_size();
    map.entities.push(MapEntity {
        kind: MapEntityKind::Unit,
        owner: "Americans".into(),
        type_id: "MTNK".into(),
        health: 256,
        x: 10,
        y: 20,
        facing: 0,
        sub_cell: 0,
    });
    let mut world = World::new(GameEdition::Ra2, &rules, map);
    assert_eq!(world.entities[0].target_x, None);
    assert!(world.entities[0].path.is_empty());
    world.push_command(GameCommand::MoveTo { entity_index: 0, x: 12, y: 20 });
    world.advance_tick();
    assert_eq!(world.entities[0].target_x, Some(12));
    assert_eq!(world.entities[0].x, 11);
    assert_eq!(world.entities[0].hva_frame, 1);
    world.advance_tick();
    assert_eq!(world.entities[0].x, 12);
    assert_eq!(world.entities[0].hva_frame, 2);
    world.advance_tick();
    assert_eq!(world.entities[0].x, 12);
}

#[test]
fn bfs_detours_around_structure() {
    let rules = rules_with_mtnk();
    let mut map = map_with_size();
    map.waypoints.push(Waypoint { index: 0, x: 14, y: 10 });
    map.entities.push(MapEntity {
        kind: MapEntityKind::Structure,
        owner: "Neutral".into(),
        type_id: "GAWALL".into(),
        health: 256,
        x: 12,
        y: 10,
        facing: 0,
        sub_cell: 0,
    });
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
    let mut world = World::new(GameEdition::Ra2, &rules, map);
    assert!(!world.pass_grid.is_passable(12, 10));
    world.push_command(GameCommand::MoveTo { entity_index: 1, x: 14, y: 10 });
    world.advance_tick();
    assert!(!world.entities[1].path.is_empty());
    assert!(!world.entities[1].path.iter().any(|&(x, y)| x == 12 && y == 10));
    // 八邻绕行仍短于直线穿墙，且不踩封死格。
    assert!(!world.entities[1].path.is_empty());
    assert!(world.entities[1].path.len() >= 3);
    for _ in 0..20 {
        world.advance_tick();
    }
    assert_eq!(world.entities[1].x, 14);
    assert_eq!(world.entities[1].y, 10);
}

#[test]
fn mobiles_detour_around_each_other() {
    let rules = rules_with_mtnk();
    let mut map = map_with_size();
    map.waypoints.push(Waypoint { index: 0, x: 14, y: 10 });
    // 挡在直线上的静止单位（Speed=0 用建筑外的占格：另一辆坦克无目标则不移动）。
    map.entities.push(MapEntity {
        kind: MapEntityKind::Unit,
        owner: "Americans".into(),
        type_id: "MTNK".into(),
        health: 256,
        x: 12,
        y: 10,
        facing: 0,
        sub_cell: 0,
    });
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
    let mut world = World::new(GameEdition::Ra2, &rules, map);
    // 两车都朝同一目标；后者路径不得踩前者当前格。
    world.push_command(GameCommand::MoveTo { entity_index: 0, x: 14, y: 10 });
    world.push_command(GameCommand::MoveTo { entity_index: 1, x: 14, y: 10 });
    world.advance_tick();
    assert!(!world.entities[1].path.is_empty());
    assert!(!world.entities[1].path.iter().any(|&(x, y)| x == world.entities[0].x && y == world.entities[0].y));
    for _ in 0..40 {
        world.advance_tick();
    }
    // 至少一车抵达或贴近目标；且不同时占同一格。
    let a = (world.entities[0].x, world.entities[0].y);
    let b = (world.entities[1].x, world.entities[1].y);
    assert_ne!(a, b);
    assert!(a == (14, 10) || b == (14, 10) || a.0.max(b.0) >= 13);
}

#[test]
fn shared_waypoint_queues_on_neighbor() {
    let rules = rules_with_mtnk();
    let mut map = map_with_size();
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
    map.entities.push(MapEntity {
        kind: MapEntityKind::Unit,
        owner: "Americans".into(),
        type_id: "MTNK".into(),
        health: 256,
        x: 10,
        y: 11,
        facing: 0,
        sub_cell: 0,
    });
    let mut world = World::new(GameEdition::Ra2, &rules, map);
    world.push_command(GameCommand::MoveTo { entity_index: 0, x: 12, y: 10 });
    world.push_command(GameCommand::MoveTo { entity_index: 1, x: 12, y: 10 });
    for _ in 0..30 {
        world.advance_tick();
    }
    let a = (world.entities[0].x, world.entities[0].y);
    let b = (world.entities[1].x, world.entities[1].y);
    assert_ne!(a, b);
    // 一车占目标，另一车停在曼哈顿距离 ≤2 的邻域。
    let on_wp = |p: (u16, u16)| p == (12, 10);
    assert!(on_wp(a) || on_wp(b));
    let other = if on_wp(a) { b } else { a };
    let dist = (i32::from(other.0) - 12).unsigned_abs() + (i32::from(other.1) - 10).unsigned_abs();
    assert!(dist <= 2);
}

#[test]
fn turret_chases_body_facing() {
    let rules = rules_with_mtnk();
    let mut map = map_with_size();
    map.waypoints.push(Waypoint { index: 0, x: 12, y: 20 });
    map.entities.push(MapEntity {
        kind: MapEntityKind::Unit,
        owner: "Americans".into(),
        type_id: "MTNK".into(),
        health: 256,
        x: 10,
        y: 20,
        facing: 0,
        sub_cell: 0,
    });
    let mut world = World::new(GameEdition::Ra2, &rules, map);
    world.entities[0].turret_facing = 128;
    world.push_command(GameCommand::MoveTo { entity_index: 0, x: 12, y: 20 });
    world.advance_tick();
    // 车身迈步后 facing 变；炮塔每 tick 最多转 TURRET_TURN_STEP。
    let body = world.entities[0].facing;
    let tur = world.entities[0].turret_facing;
    assert_ne!(tur, 128);
    let delta = (i16::from(body) - i16::from(tur)).rem_euclid(256);
    let shortest = if delta > 128 { 256 - delta } else { delta };
    assert!(shortest < 128);
}

#[test]
fn move_to_command_sets_target() {
    let rules = rules_with_mtnk();
    let mut map = map_with_size();
    map.entities.push(MapEntity {
        kind: MapEntityKind::Unit,
        owner: "Americans".into(),
        type_id: "MTNK".into(),
        health: 256,
        x: 10,
        y: 20,
        facing: 0,
        sub_cell: 0,
    });
    let mut world = World::new(GameEdition::Ra2, &rules, map);
    assert_eq!(world.entities[0].target_x, None);
    world.push_command(GameCommand::MoveTo { entity_index: 0, x: 12, y: 20 });
    world.advance_tick();
    assert_eq!(world.entities[0].target_x, Some(12));
    assert_eq!(world.entities[0].x, 11);
}

#[test]
fn attack_command_damages_and_kills() {
    let rules = rules_with_mtnk();
    let mut map = map_with_size();
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
    let mut world = World::new(GameEdition::Ra2, &rules, map);
    // 取消航点游荡，专注开火。
    world.entities[0].target_x = None;
    world.entities[0].target_y = None;
    world.entities[1].target_x = None;
    world.entities[1].target_y = None;
    world.entities[1].speed = 0;
    world.push_command(GameCommand::Attack { attacker_index: 0, target_index: 1 });
    let start_hp = world.entities[1].health;
    world.advance_tick();
    assert_eq!(world.entities[0].attack_target, Some(1));
    assert!(world.entities[1].health < start_hp);
    for _ in 0..64 {
        world.advance_tick();
        if world.entities[1].dead {
            break;
        }
    }
    assert!(world.entities[1].dead);
    assert_eq!(world.entities[1].health, 0);
    assert_eq!(world.entities[0].attack_target, None);
}

#[test]
fn every_tick_records_input_frame_including_empty() {
    let rules = rules_with_mtnk();
    let map = map_with_size();
    let mut world = World::new(GameEdition::Ra2, &rules, map);
    world.advance_tick();
    assert_eq!(world.last_input_frame().tick, 1);
    assert!(world.last_input_frame().is_empty());
    world.push_command(GameCommand::MoveTo { entity_index: 0, x: 1, y: 1 });
    // 无实体时命令被应用但帧仍记录。
    world.advance_tick();
    assert_eq!(world.last_input_frame().tick, 2);
    assert_eq!(world.last_input_frame().commands.len(), 1);
}

#[test]
fn twin_worlds_same_command_stream_match_hash() {
    let rules = rules_with_mtnk();
    let mut map = map_with_size();
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
        x: 14,
        y: 10,
        facing: 0,
        sub_cell: 0,
    });
    let mk = || {
        let mut w = World::new(GameEdition::Ra2, &rules, map.clone());
        w.entities[0].target_x = None;
        w.entities[0].target_y = None;
        w.entities[1].target_x = None;
        w.entities[1].target_y = None;
        w.entities[1].speed = 0;
        w
    };
    let mut a = mk();
    let mut b = mk();
    assert_eq!(a.state_hash(), b.state_hash());
    let cmds =
        [GameCommand::MoveTo { entity_index: 0, x: 12, y: 10 }, GameCommand::Attack { attacker_index: 0, target_index: 1 }];
    for cmd in &cmds {
        a.push_command(cmd.clone());
        b.push_command(cmd.clone());
        a.advance_tick();
        b.advance_tick();
        assert_eq!(a.state_hash(), b.state_hash());
        assert_eq!(a.last_input_frame(), b.last_input_frame());
    }
    for _ in 0..20 {
        a.advance_tick();
        b.advance_tick();
        assert_eq!(a.state_hash(), b.state_hash());
    }
}
