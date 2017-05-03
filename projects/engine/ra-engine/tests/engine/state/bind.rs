//! 规则绑定到实体运行时字段。

use crate::common::{battle_from_defs, defs_from_rules_ini, defs_with_mtnk, map_with_size};
use ra_engine::ATTACK_COOLDOWN_TICKS;
use ra_map::{MapEntity, MapEntityKind, MapHouse, MapInfo, OverlayCell, Waypoint};
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
