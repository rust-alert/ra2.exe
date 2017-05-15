//! 规则绑定到实体运行时字段。

use crate::common::{battle_from_defs, defs_from_rules_ini, defs_with_mtnk, map_with_size};
use ra_engine::ATTACK_COOLDOWN_TICKS;
use ra_map::{
    MapAction, MapActionCommand, MapActionKind, MapAiTrigger, MapCellTag, MapEntity, MapEntityKind, MapEvent, MapEventCondition, MapEventKind,
    MapHouse, MapInfo, MapScriptStep, MapScriptType, MapTag, MapTaskForce, MapTaskForceEntry, MapTeamType, MapTrigger, OverlayCell, Waypoint,
};
use ra_types::{GameEdition, MapEdge, MapPlacedEntityKind, TechnoClass, occupancy_kind};

#[test]
fn binds_strength_and_speed() {
    let defs = defs_with_mtnk();
    let mut map = map_with_size();
    map.entities.push(MapEntity {
        kind: MapEntityKind::Unit,
        owner: "AMERICANS".into(),
        type_id: "MTNK".into(),
        health: 256,
        x: 10,
        y: 20,
        facing: 64,
        sub_cell: 0,
        mission: Default::default(),
        tag: Default::default(),
    });
    let world = battle_from_defs(GameEdition::Ra2, defs, map);
    assert_eq!(world.entity_count(), 1);
    let id = world.entity_id_at(0).expect("entity");
    let health = world.ecs_health(id).expect("health");
    let combat = world.ecs_combat_view(id).expect("combat");
    assert_eq!(health.1, 400);
    assert_eq!(health.0, 400);
    assert_eq!(world.ecs_speed(id), Some(64));
    assert_eq!(combat.attack_range, 6);
    assert_eq!(combat.attack_damage, 100);
    assert_eq!(combat.attack_cooldown_max, ATTACK_COOLDOWN_TICKS);
    assert_eq!(combat.techno_class, Some(TechnoClass::Vehicle));
    assert_eq!(world.bound_techno_count(), 1);
}

#[test]
fn unbound_techno_rejects_battle_seed() {
    let defs = defs_with_mtnk();
    let mut map = map_with_size();
    map.entities.push(MapEntity {
        kind: MapEntityKind::Unit,
        owner: "AMERICANS".into(),
        type_id: "NOSUCH".into(),
        health: 256,
        x: 10,
        y: 20,
        facing: 0,
        sub_cell: 0,
        mission: Default::default(),
        tag: Default::default(),
    });
    let err = ra_engine::BattleState::new(GameEdition::Ra2, defs, map).expect_err("unknown techno must fail seed");
    let msg = err.to_string();
    assert!(msg.contains("techno") || msg.contains("NOSUCH"), "{msg}");
}

#[test]
fn unbound_house_rejects_battle_seed() {
    let defs = defs_with_mtnk();
    let mut map = map_with_size();
    map.entities.push(MapEntity {
        kind: MapEntityKind::Unit,
        owner: "NOSUCHHOUSE".into(),
        type_id: "MTNK".into(),
        health: 256,
        x: 10,
        y: 20,
        facing: 0,
        sub_cell: 0,
        mission: Default::default(),
        tag: Default::default(),
    });
    let err = ra_engine::BattleState::new(GameEdition::Ra2, defs, map).expect_err("unknown house must fail seed");
    let msg = err.to_string();
    assert!(msg.contains("house") || msg.contains("NOSUCHHOUSE"), "{msg}");
}

#[test]
fn seeds_structure_health_from_map_ratio() {
    let defs = defs_from_rules_ini(
        b"[BuildingTypes]\n0=CAGAS01\n\
[CAGAS01]\nStrength=1000\nSight=4\nCost=100\nArmor=wood\n",
    );
    let mut map = map_with_size();
    map.entities.push(MapEntity {
        kind: MapEntityKind::Structure,
        owner: "NEUTRAL".into(),
        type_id: "CAGAS01".into(),
        health: 64,
        x: 8,
        y: 8,
        facing: 0,
        sub_cell: 0,
        mission: Default::default(),
        tag: Default::default(),
    });
    let world = battle_from_defs(GameEdition::Ra2, defs, map);
    let id = world.entity_id_at(0).expect("entity");
    let health = world.ecs_health(id).expect("health");
    assert_eq!(health.1, 1000);
    assert_eq!(health.0, 250, "64/256 of Strength=1000");
    assert!(!health.2);
}

/// Map-3：`MapInfo` → bind → `PreparedPlacement` → Foundation 占格 → ECS spawn 闭环。
#[test]
fn prepared_map_seed_binds_ids_foundation_house_waypoint_and_overlay() {
    let defs = defs_from_rules_ini(
        b"[VehicleTypes]\n0=MTNK\n\
[InfantryTypes]\n0=E1\n\
[AircraftTypes]\n0=ORCA\n\
[BuildingTypes]\n0=GAPOWR\n\
[MTNK]\nStrength=400\nSpeed=64\nSight=6\nCost=800\nArmor=heavy\nPrimary=90mm\n\
[E1]\nStrength=125\nSpeed=32\nSight=5\nCost=200\nArmor=none\n\
[ORCA]\nStrength=200\nSpeed=100\nSight=8\nCost=1000\nArmor=light\n\
[GAPOWR]\nPower=200\nOwner=Americans\nStrength=600\nSight=4\nCost=600\nTechLevel=1\nFoundation=2x2\n\
[90mm]\nDamage=100\nROF=8\nRange=6\nWarhead=SA\n\
[SA]\nVerses=100%,100%,100%,100%,100%,100%,100%,100%,100%,100%,100%\n",
    );
    let gapowr = defs.techno.get("GAPOWR").expect("GAPOWR").id;
    let mtnk = defs.techno.get("MTNK").expect("MTNK").id;
    let e1 = defs.techno.get("E1").expect("E1").id;
    let orca = defs.techno.get("ORCA").expect("ORCA").id;
    let americans = defs.houses.get("AMERICANS").expect("AMERICANS").id;
    let russians = defs.houses.get("RUSSIANS").expect("RUSSIANS").id;

    let mut map = MapInfo::empty(GameEdition::Ra2, "map3-bind");
    map.width = 16;
    map.height = 16;
    map.entities = vec![
        MapEntity {
            kind: MapEntityKind::Structure,
            owner: "AMERICANS".into(),
            type_id: "GAPOWR".into(),
            health: 256,
            x: 4,
            y: 4,
            facing: 0,
            sub_cell: 0,
            mission: Default::default(),
            tag: Default::default(),
        },
        MapEntity {
            kind: MapEntityKind::Unit,
            owner: "AMERICANS".into(),
            type_id: "MTNK".into(),
            health: 256,
            x: 8,
            y: 4,
            facing: 32,
            sub_cell: 0,
            mission: Default::default(),
            tag: Default::default(),
        },
        MapEntity {
            kind: MapEntityKind::Infantry,
            owner: "RUSSIANS".into(),
            type_id: "E1".into(),
            health: 256,
            x: 10,
            y: 6,
            facing: 0,
            sub_cell: 2,
            mission: Default::default(),
            tag: Default::default(),
        },
        MapEntity {
            kind: MapEntityKind::Aircraft,
            owner: "AMERICANS".into(),
            type_id: "ORCA".into(),
            health: 256,
            x: 12,
            y: 8,
            facing: 16,
            sub_cell: 0,
            mission: Default::default(),
            tag: Default::default(),
        },
    ];
    map.waypoints.push(Waypoint { index: 0, x: 2, y: 3 });
    map.overlays.push(OverlayCell { x: 1, y: 1, overlay_id: 7, data: 0 });
    map.scripting.houses.push(MapHouse {
        name: "Player House".into(),
        country: "Americans".into(),
        tech_level: 10,
        credits: 50,
        iq: 0,
        edge: MapEdge::North,
        player_control: true,
        color: "Gold".into(),
        allies: vec!["Alliance".into()],
    });

    let world = battle_from_defs(GameEdition::Ra2, defs.clone(), map);
    assert_eq!(world.entity_count(), 4);
    assert_eq!(world.prepared.placements.len(), 4);

    let structure = world.prepared.placements.iter().find(|p| p.kind == MapPlacedEntityKind::Structure).expect("structure placement");
    assert_eq!(structure.definition_id, gapowr);
    assert_eq!(structure.owner, americans);
    assert_eq!((structure.x, structure.y), (4, 4));

    let unit = world.prepared.placements.iter().find(|p| p.kind == MapPlacedEntityKind::Unit).expect("unit placement");
    assert_eq!(unit.definition_id, mtnk);
    assert_eq!(unit.owner, americans);

    let infantry = world.prepared.placements.iter().find(|p| p.kind == MapPlacedEntityKind::Infantry).expect("infantry placement");
    assert_eq!(infantry.definition_id, e1);
    assert_eq!(infantry.owner, russians);
    assert_eq!(infantry.sub_cell, 2);

    let aircraft = world.prepared.placements.iter().find(|p| p.kind == MapPlacedEntityKind::Aircraft).expect("aircraft placement");
    assert_eq!(aircraft.definition_id, orca);
    assert_eq!(aircraft.owner, americans);
    assert_eq!((aircraft.x, aircraft.y), (12, 8));

    let idx = |x: u16, y: u16| (y as usize) * (world.prepared.pass_width as usize) + (x as usize);
    assert_eq!(world.prepared.occupancy[idx(4, 4)], occupancy_kind::STRUCTURE);
    assert_eq!(world.prepared.occupancy[idx(5, 5)], occupancy_kind::STRUCTURE);
    assert_eq!(world.prepared.passable[idx(4, 4)], 0);
    assert_eq!(world.prepared.passable[idx(5, 5)], 0);
    assert!(!world.pass_grid.is_passable(4, 4));
    assert!(!world.pass_grid.is_passable(5, 5));
    assert!(world.pass_grid.is_passable(6, 6));

    assert_eq!(world.prepared.definition.waypoints.len(), 1);
    assert_eq!(world.prepared.definition.waypoints[0], ra_types::MapWaypoint { index: 0, x: 2, y: 3 });
    assert_eq!(world.prepared.definition.overlays.len(), 1);
    assert_eq!(world.prepared.definition.overlays[0].overlay_id, 7);

    assert_eq!(world.prepared.houses.len(), 1);
    assert_eq!(world.prepared.houses[0].name, "Player House");
    assert_eq!(world.prepared.houses[0].country, americans);
    assert_eq!(world.prepared.houses[0].credits, 50);
    assert_eq!(world.prepared.houses[0].allies, vec![defs.houses.get("ALLIANCE").expect("ALLIANCE").id]);

    let power_id = world.entity_id_at(0).expect("structure entity");
    let identity = world.ecs_identity(power_id).expect("identity");
    assert_eq!(identity.0.as_ref(), "GAPOWR");
    assert_eq!(identity.1, MapEntityKind::Structure);
    assert_eq!(world.ecs_owner(power_id).expect("owner").as_ref(), "AMERICANS");
}

#[test]
fn prepared_map_seed_binds_mission_kind() {
    let defs = defs_with_mtnk();
    let mut map = map_with_size();
    map.entities.push(MapEntity {
        kind: MapEntityKind::Unit,
        owner: "AMERICANS".into(),
        type_id: "MTNK".into(),
        health: 256,
        x: 10,
        y: 20,
        facing: 0,
        sub_cell: 0,
        mission: "Guard".into(),
        tag: Default::default(),
    });
    let world = battle_from_defs(GameEdition::Ra2, defs, map);
    assert_eq!(world.prepared.placements[0].mission, Some(ra_types::MissionKind::Guard));
    assert_eq!(world.entity_count(), 1);
}

#[test]
fn unbound_mission_rejects_battle_seed() {
    let defs = defs_with_mtnk();
    let mut map = map_with_size();
    map.entities.push(MapEntity {
        kind: MapEntityKind::Unit,
        owner: "AMERICANS".into(),
        type_id: "MTNK".into(),
        health: 256,
        x: 10,
        y: 20,
        facing: 0,
        sub_cell: 0,
        mission: "NotAMission".into(),
        tag: Default::default(),
    });
    let err = ra_engine::BattleState::new(GameEdition::Ra2, defs, map).expect_err("unknown mission must fail seed");
    let msg = err.to_string();
    assert!(msg.contains("mission") || msg.contains("NotAMission") || msg.contains("NOTAMISSION"), "{msg}");
}

#[test]
fn prepared_map_seed_binds_tag_id() {
    let defs = defs_with_mtnk();
    let mut map = map_with_size();
    map.scripting.triggers.push(MapTrigger {
        id: "TR1".into(),
        house: "Americans".into(),
        linked: Default::default(),
        name: "Trig".into(),
        disabled: false,
        easy: true,
        normal: true,
        hard: true,
    });
    map.scripting.tags.push(MapTag { id: "T1".into(), persistence: 0, name: "Start".into(), trigger_id: "TR1".into() });
    map.entities.push(MapEntity {
        kind: MapEntityKind::Unit,
        owner: "AMERICANS".into(),
        type_id: "MTNK".into(),
        health: 256,
        x: 10,
        y: 20,
        facing: 0,
        sub_cell: 0,
        mission: Default::default(),
        tag: "T1".into(),
    });
    let world = battle_from_defs(GameEdition::Ra2, defs, map);
    assert_eq!(world.prepared.tags.len(), 1);
    assert_eq!(world.prepared.triggers.len(), 1);
    assert_eq!(world.prepared.placements[0].tag, Some(world.prepared.tags[0].id));
    assert_eq!(world.prepared.tags[0].trigger_id, world.prepared.triggers[0].id);
}

#[test]
fn prepared_map_seed_accepts_none_tag_sentinel() {
    let defs = defs_with_mtnk();
    let mut map = map_with_size();
    map.entities.push(MapEntity {
        kind: MapEntityKind::Unit,
        owner: "AMERICANS".into(),
        type_id: "MTNK".into(),
        health: 256,
        x: 10,
        y: 20,
        facing: 0,
        sub_cell: 0,
        mission: Default::default(),
        tag: "None".into(),
    });
    let world = battle_from_defs(GameEdition::Ra2, defs, map);
    assert_eq!(world.prepared.placements[0].tag, None);
}

#[test]
fn unbound_tag_rejects_battle_seed() {
    let defs = defs_with_mtnk();
    let mut map = map_with_size();
    map.entities.push(MapEntity {
        kind: MapEntityKind::Unit,
        owner: "AMERICANS".into(),
        type_id: "MTNK".into(),
        health: 256,
        x: 10,
        y: 20,
        facing: 0,
        sub_cell: 0,
        mission: Default::default(),
        tag: "MissingTag".into(),
    });
    let err = ra_engine::BattleState::new(GameEdition::Ra2, defs, map).expect_err("unknown tag must fail seed");
    let msg = err.to_string();
    assert!(msg.contains("tag") || msg.contains("MissingTag") || msg.contains("MISSINGTAG"), "{msg}");
}

#[test]
fn prepared_map_seed_binds_cell_tag_id() {
    let defs = defs_with_mtnk();
    let mut map = map_with_size();
    map.scripting.triggers.push(MapTrigger {
        id: "TR1".into(),
        house: "Americans".into(),
        linked: Default::default(),
        name: "Trig".into(),
        disabled: false,
        easy: true,
        normal: true,
        hard: true,
    });
    map.scripting.tags.push(MapTag { id: "T1".into(), persistence: 0, name: "Cell".into(), trigger_id: "TR1".into() });
    map.scripting.cell_tags.push(MapCellTag { x: 3, y: 5, tag_id: "T1".into() });
    let world = battle_from_defs(GameEdition::Ra2, defs, map);
    assert_eq!(world.prepared.cell_tags.len(), 1);
    assert_eq!(world.prepared.cell_tags[0].x, 3);
    assert_eq!(world.prepared.cell_tags[0].y, 5);
    assert_eq!(world.prepared.cell_tags[0].tag, world.prepared.tags[0].id);
}

#[test]
fn unbound_cell_tag_rejects_battle_seed() {
    let defs = defs_with_mtnk();
    let mut map = map_with_size();
    map.scripting.cell_tags.push(MapCellTag { x: 1, y: 1, tag_id: "MissingTag".into() });
    let err = ra_engine::BattleState::new(GameEdition::Ra2, defs, map).expect_err("unknown cell tag must fail seed");
    let msg = err.to_string();
    assert!(msg.contains("tag") || msg.contains("MissingTag") || msg.contains("MISSINGTAG"), "{msg}");
}

#[test]
fn prepared_map_seed_binds_task_force_techno_ids() {
    let defs = defs_with_mtnk();
    let mtnk = defs.techno.get("MTNK").expect("MTNK").id;
    let mut map = map_with_size();
    map.scripting.task_forces.push(MapTaskForce {
        id: "TF1".into(),
        name: "Armor".into(),
        entries: vec![MapTaskForceEntry { count: 2, type_id: "MTNK".into() }],
        group: -1,
    });
    let world = battle_from_defs(GameEdition::Ra2, defs, map);
    assert_eq!(world.prepared.task_forces.len(), 1);
    assert_eq!(world.prepared.task_forces[0].entries.len(), 1);
    assert_eq!(world.prepared.task_forces[0].entries[0].count, 2);
    assert_eq!(world.prepared.task_forces[0].entries[0].definition_id, mtnk);
}

#[test]
fn unbound_task_force_techno_rejects_battle_seed() {
    let defs = defs_with_mtnk();
    let mut map = map_with_size();
    map.scripting.task_forces.push(MapTaskForce {
        id: "TF1".into(),
        name: "Bad".into(),
        entries: vec![MapTaskForceEntry { count: 1, type_id: "NOSUCH".into() }],
        group: -1,
    });
    let err = ra_engine::BattleState::new(GameEdition::Ra2, defs, map).expect_err("unknown task force techno must fail seed");
    let msg = err.to_string();
    assert!(msg.contains("techno") || msg.contains("NOSUCH"), "{msg}");
}

#[test]
fn prepared_map_seed_binds_team_type_refs() {
    let defs = defs_with_mtnk();
    let americans = defs.houses.get("AMERICANS").expect("AMERICANS").id;
    let mut map = map_with_size();
    map.scripting.task_forces.push(MapTaskForce {
        id: "TF1".into(),
        name: "Armor".into(),
        entries: vec![MapTaskForceEntry { count: 1, type_id: "MTNK".into() }],
        group: -1,
    });
    map.scripting.script_types.push(MapScriptType {
        id: "SC1".into(),
        name: "Move".into(),
        steps: vec![MapScriptStep { action: 3, argument: 0 }],
    });
    map.scripting.team_types.push(MapTeamType {
        id: "TM1".into(),
        name: "TankTeam".into(),
        house: "Americans".into(),
        script: "SC1".into(),
        task_force: "TF1".into(),
        tag: Default::default(),
        waypoint: 0,
        max: 1,
        priority: 10,
        veteran_level: 0,
    });
    let world = battle_from_defs(GameEdition::Ra2, defs, map);
    assert_eq!(world.prepared.team_types.len(), 1);
    assert_eq!(world.prepared.script_types.len(), 1);
    assert_eq!(world.prepared.task_forces.len(), 1);
    let team = &world.prepared.team_types[0];
    assert_eq!(team.house, americans);
    assert_eq!(team.script, Some(world.prepared.script_types[0].id));
    assert_eq!(team.task_force, world.prepared.task_forces[0].id);
    assert_eq!(team.tag, None);
}

#[test]
fn unbound_team_type_task_force_rejects_battle_seed() {
    let defs = defs_with_mtnk();
    let mut map = map_with_size();
    map.scripting.script_types.push(MapScriptType {
        id: "SC1".into(),
        name: "Move".into(),
        steps: vec![MapScriptStep { action: 3, argument: 0 }],
    });
    map.scripting.team_types.push(MapTeamType {
        id: "TM1".into(),
        name: "Bad".into(),
        house: "Americans".into(),
        script: "SC1".into(),
        task_force: "MissingTF".into(),
        tag: Default::default(),
        waypoint: 0,
        max: 1,
        priority: 10,
        veteran_level: 0,
    });
    let err = ra_engine::BattleState::new(GameEdition::Ra2, defs, map).expect_err("unknown team task force must fail seed");
    let msg = err.to_string();
    assert!(msg.contains("task_force") || msg.contains("MissingTF") || msg.contains("MISSINGTF"), "{msg}");
}

#[test]
fn prepared_map_seed_binds_ai_trigger_team_and_house() {
    let defs = defs_with_mtnk();
    let americans = defs.houses.get("AMERICANS").expect("AMERICANS").id;
    let mut map = map_with_size();
    map.scripting.task_forces.push(MapTaskForce {
        id: "TF1".into(),
        name: "Armor".into(),
        entries: vec![MapTaskForceEntry { count: 1, type_id: "MTNK".into() }],
        group: -1,
    });
    map.scripting.team_types.push(MapTeamType {
        id: "TM1".into(),
        name: "TankTeam".into(),
        house: "Americans".into(),
        script: Default::default(),
        task_force: "TF1".into(),
        tag: Default::default(),
        waypoint: -1,
        max: 1,
        priority: 0,
        veteran_level: 0,
    });
    map.scripting.ai_triggers.push(MapAiTrigger {
        id: "AI1".into(),
        name: "Spawn".into(),
        team: "TM1".into(),
        owner_house: "Americans".into(),
        tech_level: 1,
        weight: 50,
        ..Default::default()
    });
    let world = battle_from_defs(GameEdition::Ra2, defs, map);
    assert_eq!(world.prepared.ai_triggers.len(), 1);
    assert_eq!(world.prepared.ai_triggers[0].team, world.prepared.team_types[0].id);
    assert_eq!(world.prepared.ai_triggers[0].owner_house, Some(americans));
    assert_eq!(world.prepared.ai_triggers[0].tech_level, 1);
    assert_eq!(world.prepared.ai_triggers[0].weight, 50);
}

#[test]
fn unbound_ai_trigger_team_rejects_battle_seed() {
    let defs = defs_with_mtnk();
    let mut map = map_with_size();
    map.scripting.ai_triggers.push(MapAiTrigger {
        id: "AI1".into(),
        name: "Broken".into(),
        team: "MissingTeam".into(),
        owner_house: "Americans".into(),
        ..Default::default()
    });
    let err = ra_engine::BattleState::new(GameEdition::Ra2, defs, map).expect_err("unknown ai trigger team must fail seed");
    let msg = err.to_string();
    assert!(msg.contains("team_type") || msg.contains("MissingTeam") || msg.contains("MISSINGTEAM"), "{msg}");
}

#[test]
fn prepared_map_seed_binds_events_and_actions_to_trigger() {
    let defs = defs_with_mtnk();
    let mut map = map_with_size();
    map.scripting.triggers.push(MapTrigger {
        id: "TR1".into(),
        house: "Americans".into(),
        linked: Default::default(),
        name: "Timer".into(),
        disabled: false,
        easy: true,
        normal: true,
        hard: true,
    });
    map.scripting.events.push(MapEvent {
        id: "TR1".into(),
        conditions: vec![MapEventCondition { kind: MapEventKind::TimeElapse, params: vec!["10".into(), "0".into()] }],
    });
    map.scripting.actions.push(MapAction {
        id: "TR1".into(),
        commands: vec![MapActionCommand {
            kind: MapActionKind::from_code(1),
            params: ["Americans".into(), String::new(), String::new(), String::new(), String::new(), String::new(), String::new()],
        }],
    });
    let world = battle_from_defs(GameEdition::Ra2, defs, map);
    assert_eq!(world.prepared.triggers.len(), 1);
    assert_eq!(world.prepared.events.len(), 1);
    assert_eq!(world.prepared.actions.len(), 1);
    assert_eq!(world.prepared.events[0].trigger_id, world.prepared.triggers[0].id);
    assert_eq!(world.prepared.actions[0].trigger_id, world.prepared.triggers[0].id);
    assert_eq!(world.prepared.events[0].conditions[0].kind_code, MapEventKind::TimeElapse.code());
}

#[test]
fn unbound_event_trigger_rejects_battle_seed() {
    let defs = defs_with_mtnk();
    let mut map = map_with_size();
    map.scripting.events.push(MapEvent { id: "MissingTR".into(), conditions: Vec::new() });
    let err = ra_engine::BattleState::new(GameEdition::Ra2, defs, map).expect_err("unknown event trigger must fail seed");
    let msg = err.to_string();
    assert!(msg.contains("trigger") || msg.contains("MissingTR") || msg.contains("MISSINGTR"), "{msg}");
}

#[test]
fn prepared_map_seed_binds_create_team_action_id() {
    let defs = defs_with_mtnk();
    let mut map = map_with_size();
    map.scripting.triggers.push(MapTrigger {
        id: "TR1".into(),
        house: "Americans".into(),
        linked: Default::default(),
        name: "Spawn".into(),
        disabled: false,
        easy: true,
        normal: true,
        hard: true,
    });
    map.scripting.task_forces.push(MapTaskForce {
        id: "TF1".into(),
        name: "Armor".into(),
        entries: vec![MapTaskForceEntry { count: 1, type_id: "MTNK".into() }],
        group: -1,
    });
    map.scripting.team_types.push(MapTeamType {
        id: "TM1".into(),
        name: "TankTeam".into(),
        house: "Americans".into(),
        script: Default::default(),
        task_force: "TF1".into(),
        tag: Default::default(),
        waypoint: -1,
        max: 1,
        priority: 0,
        veteran_level: 0,
    });
    map.scripting.actions.push(MapAction {
        id: "TR1".into(),
        commands: vec![MapActionCommand {
            kind: MapActionKind::CreateTeam,
            params: ["0".into(), "TM1".into(), String::new(), String::new(), String::new(), String::new(), String::new()],
        }],
    });
    let world = battle_from_defs(GameEdition::Ra2, defs, map);
    assert_eq!(world.prepared.actions.len(), 1);
    assert_eq!(world.prepared.actions[0].commands[0].team_id, Some(world.prepared.team_types[0].id));
}

#[test]
fn unbound_create_team_action_rejects_battle_seed() {
    let defs = defs_with_mtnk();
    let mut map = map_with_size();
    map.scripting.triggers.push(MapTrigger {
        id: "TR1".into(),
        house: "Americans".into(),
        linked: Default::default(),
        name: "Spawn".into(),
        disabled: false,
        easy: true,
        normal: true,
        hard: true,
    });
    map.scripting.actions.push(MapAction {
        id: "TR1".into(),
        commands: vec![MapActionCommand {
            kind: MapActionKind::CreateTeam,
            params: ["0".into(), "MissingTeam".into(), String::new(), String::new(), String::new(), String::new(), String::new()],
        }],
    });
    let err = ra_engine::BattleState::new(GameEdition::Ra2, defs, map).expect_err("unknown CreateTeam must fail seed");
    let msg = err.to_string();
    assert!(msg.contains("team_type") || msg.contains("MissingTeam") || msg.contains("MISSINGTEAM"), "{msg}");
}

fn seed_map_with_team(defs_house: &str) -> (MapInfo, String) {
    let mut map = map_with_size();
    map.scripting.triggers.push(MapTrigger {
        id: "TR1".into(),
        house: defs_house.into(),
        linked: Default::default(),
        name: "Act".into(),
        disabled: false,
        easy: true,
        normal: true,
        hard: true,
    });
    map.scripting.task_forces.push(MapTaskForce {
        id: "TF1".into(),
        name: "Armor".into(),
        entries: vec![MapTaskForceEntry { count: 1, type_id: "MTNK".into() }],
        group: -1,
    });
    map.scripting.team_types.push(MapTeamType {
        id: "TM1".into(),
        name: "TankTeam".into(),
        house: defs_house.into(),
        script: Default::default(),
        task_force: "TF1".into(),
        tag: Default::default(),
        waypoint: -1,
        max: 1,
        priority: 0,
        veteran_level: 0,
    });
    (map, "TM1".into())
}

#[test]
fn prepared_map_seed_binds_destroy_and_reinforcement_team_actions() {
    let defs = defs_with_mtnk();
    let (mut map, team) = seed_map_with_team("Americans");
    map.scripting.actions.push(MapAction {
        id: "TR1".into(),
        commands: vec![
            MapActionCommand {
                kind: MapActionKind::DestroyTeam,
                params: ["0".into(), team.clone(), String::new(), String::new(), String::new(), String::new(), String::new()],
            },
            MapActionCommand {
                kind: MapActionKind::Reinforcement,
                params: ["0".into(), team, String::new(), String::new(), String::new(), String::new(), String::new()],
            },
        ],
    });
    let world = battle_from_defs(GameEdition::Ra2, defs, map);
    assert_eq!(world.prepared.actions[0].commands.len(), 2);
    let team_id = world.prepared.team_types[0].id;
    assert_eq!(world.prepared.actions[0].commands[0].team_id, Some(team_id));
    assert_eq!(world.prepared.actions[0].commands[1].team_id, Some(team_id));
}

#[test]
fn prepared_map_seed_binds_force_trigger_action_id() {
    let defs = defs_with_mtnk();
    let mut map = map_with_size();
    map.scripting.triggers.push(MapTrigger {
        id: "TR1".into(),
        house: "Americans".into(),
        linked: Default::default(),
        name: "Src".into(),
        disabled: false,
        easy: true,
        normal: true,
        hard: true,
    });
    map.scripting.triggers.push(MapTrigger {
        id: "TR2".into(),
        house: "Americans".into(),
        linked: Default::default(),
        name: "Dst".into(),
        disabled: false,
        easy: true,
        normal: true,
        hard: true,
    });
    map.scripting.actions.push(MapAction {
        id: "TR1".into(),
        commands: vec![MapActionCommand {
            kind: MapActionKind::ForceTrigger,
            params: ["0".into(), "TR2".into(), String::new(), String::new(), String::new(), String::new(), String::new()],
        }],
    });
    let world = battle_from_defs(GameEdition::Ra2, defs, map);
    let tr2 = world.prepared.triggers.iter().find(|t| t.name.as_str() == "TR2").expect("TR2");
    assert_eq!(world.prepared.actions[0].commands[0].target_trigger_id, Some(tr2.id));
}

#[test]
fn unbound_force_trigger_action_rejects_battle_seed() {
    let defs = defs_with_mtnk();
    let mut map = map_with_size();
    map.scripting.triggers.push(MapTrigger {
        id: "TR1".into(),
        house: "Americans".into(),
        linked: Default::default(),
        name: "Src".into(),
        disabled: false,
        easy: true,
        normal: true,
        hard: true,
    });
    map.scripting.actions.push(MapAction {
        id: "TR1".into(),
        commands: vec![MapActionCommand {
            kind: MapActionKind::ForceTrigger,
            params: ["0".into(), "MissingTR".into(), String::new(), String::new(), String::new(), String::new(), String::new()],
        }],
    });
    let err = ra_engine::BattleState::new(GameEdition::Ra2, defs, map).expect_err("unknown ForceTrigger must fail seed");
    let msg = err.to_string();
    assert!(msg.contains("trigger") || msg.contains("MissingTR") || msg.contains("MISSINGTR"), "{msg}");
}

#[test]
fn prepared_map_seed_binds_destroy_tag_action_id() {
    let defs = defs_with_mtnk();
    let mut map = map_with_size();
    map.scripting.triggers.push(MapTrigger {
        id: "TR1".into(),
        house: "Americans".into(),
        linked: Default::default(),
        name: "Act".into(),
        disabled: false,
        easy: true,
        normal: true,
        hard: true,
    });
    map.scripting.tags.push(MapTag { id: "T1".into(), persistence: 0, name: "Tagged".into(), trigger_id: "TR1".into() });
    map.scripting.actions.push(MapAction {
        id: "TR1".into(),
        commands: vec![MapActionCommand {
            kind: MapActionKind::DestroyTag,
            params: ["0".into(), "T1".into(), String::new(), String::new(), String::new(), String::new(), String::new()],
        }],
    });
    let world = battle_from_defs(GameEdition::Ra2, defs, map);
    assert_eq!(world.prepared.actions[0].commands[0].tag_id, Some(world.prepared.tags[0].id));
}

#[test]
fn unbound_destroy_tag_action_rejects_battle_seed() {
    let defs = defs_with_mtnk();
    let mut map = map_with_size();
    map.scripting.triggers.push(MapTrigger {
        id: "TR1".into(),
        house: "Americans".into(),
        linked: Default::default(),
        name: "Act".into(),
        disabled: false,
        easy: true,
        normal: true,
        hard: true,
    });
    map.scripting.actions.push(MapAction {
        id: "TR1".into(),
        commands: vec![MapActionCommand {
            kind: MapActionKind::DestroyTag,
            params: ["0".into(), "MissingTag".into(), String::new(), String::new(), String::new(), String::new(), String::new()],
        }],
    });
    let err = ra_engine::BattleState::new(GameEdition::Ra2, defs, map).expect_err("unknown DestroyTag must fail seed");
    let msg = err.to_string();
    assert!(msg.contains("tag") || msg.contains("MissingTag") || msg.contains("MISSINGTAG"), "{msg}");
}

#[test]
fn prepared_map_seed_binds_enable_and_disable_trigger_actions() {
    let defs = defs_with_mtnk();
    let mut map = map_with_size();
    map.scripting.triggers.push(MapTrigger {
        id: "TR1".into(),
        house: "Americans".into(),
        linked: Default::default(),
        name: "Src".into(),
        disabled: false,
        easy: true,
        normal: true,
        hard: true,
    });
    map.scripting.triggers.push(MapTrigger {
        id: "TR2".into(),
        house: "Americans".into(),
        linked: Default::default(),
        name: "Dst".into(),
        disabled: true,
        easy: true,
        normal: true,
        hard: true,
    });
    map.scripting.actions.push(MapAction {
        id: "TR1".into(),
        commands: vec![
            MapActionCommand {
                kind: MapActionKind::EnableTrigger,
                params: ["0".into(), "TR2".into(), String::new(), String::new(), String::new(), String::new(), String::new()],
            },
            MapActionCommand {
                kind: MapActionKind::DisableTrigger,
                params: ["0".into(), "TR2".into(), String::new(), String::new(), String::new(), String::new(), String::new()],
            },
        ],
    });
    let world = battle_from_defs(GameEdition::Ra2, defs, map);
    let tr2 = world.prepared.triggers.iter().find(|t| t.name.as_str() == "TR2").expect("TR2");
    assert_eq!(world.prepared.actions[0].commands[0].target_trigger_id, Some(tr2.id));
    assert_eq!(world.prepared.actions[0].commands[1].target_trigger_id, Some(tr2.id));
}

#[test]
fn prepared_map_seed_binds_linked_trigger_id() {
    let defs = defs_with_mtnk();
    let mut map = map_with_size();
    map.scripting.triggers.push(MapTrigger {
        id: "TR1".into(),
        house: "Americans".into(),
        linked: Default::default(),
        name: "First".into(),
        disabled: false,
        easy: true,
        normal: true,
        hard: true,
    });
    map.scripting.triggers.push(MapTrigger {
        id: "TR2".into(),
        house: "Americans".into(),
        linked: "TR1".into(),
        name: "Second".into(),
        disabled: false,
        easy: true,
        normal: true,
        hard: true,
    });
    let world = battle_from_defs(GameEdition::Ra2, defs, map);
    assert_eq!(world.prepared.triggers.len(), 2);
    let tr1 = world.prepared.triggers.iter().find(|t| t.name.as_str() == "TR1").expect("TR1");
    let tr2 = world.prepared.triggers.iter().find(|t| t.name.as_str() == "TR2").expect("TR2");
    assert_eq!(tr1.linked, None);
    assert_eq!(tr2.linked, Some(tr1.id));
}

#[test]
fn unbound_linked_trigger_rejects_battle_seed() {
    let defs = defs_with_mtnk();
    let mut map = map_with_size();
    map.scripting.triggers.push(MapTrigger {
        id: "TR1".into(),
        house: "Americans".into(),
        linked: "MissingTR".into(),
        name: "Broken".into(),
        disabled: false,
        easy: true,
        normal: true,
        hard: true,
    });
    let err = ra_engine::BattleState::new(GameEdition::Ra2, defs, map).expect_err("unknown linked trigger must fail seed");
    let msg = err.to_string();
    assert!(msg.contains("trigger") || msg.contains("MissingTR") || msg.contains("MISSINGTR"), "{msg}");
}
