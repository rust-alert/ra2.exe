//! Map-3：会话装载走 `to_prepared_map` → spawn → `finalize_pass_from_assets` 闭环。

use crate::common::defs_from_rules_ini;
use ra_engine::{open_campaign_session_prepared, validate_map_for_battle};
use ra_map::{MapEntity, MapEntityKind, MapHouse, MapInfo, OverlayCell, Waypoint};
use ra_types::{AssetSource, ColorName, GameEdition, HouseName, MapEdge, RaError, RaResult, occupancy_kind};

const RULES_INI: &str = "rules.ini";

fn prepare_defs() -> std::sync::Arc<ra_types::RuntimeDefinitions> {
    defs_from_rules_ini(
        b"[Countries]\n0=Americans\n1=Russians\n\
[Americans]\nSide=GDI\n\
[Russians]\nSide=Nod\n\
[OverlayTypes]\n0=LOBRDG01\n\
[LOBRDG01]\nLand=Road\nNoUseTileLandType=yes\n\
[InfantryTypes]\n0=E1\n\
[VehicleTypes]\n0=MTNK\n\
[AircraftTypes]\n0=ORCA\n\
[BuildingTypes]\n0=GAPOWR\n\
[E1]\nOwner=Americans\nStrength=125\nSpeed=24\nSight=4\nCost=200\n\
[MTNK]\nOwner=Americans\nStrength=400\nSpeed=64\nSight=6\nCost=800\n\
[ORCA]\nOwner=Americans\nStrength=200\nSpeed=120\nSight=8\nCost=1000\n\
[GAPOWR]\nPower=200\nOwner=Americans\nStrength=600\nSight=4\nCost=600\nTechLevel=1\nFoundation=2x2\n",
    )
}

fn prepare_map() -> MapInfo {
    let mut map = MapInfo::empty(GameEdition::Ra2, "map3-session");
    map.width = 16;
    map.height = 16;
    map.waypoints = vec![Waypoint { index: 0, x: 2, y: 2 }];
    map.scripting.houses.push(MapHouse {
        name: "Player House".into(),
        country: HouseName::parse("Americans"),
        tech_level: 10,
        credits: 50,
        iq: 0,
        edge: MapEdge::North,
        player_control: true,
        color: ColorName::parse("Gold"),
        allies: vec![],
    });
    map.entities.push(MapEntity {
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
    });
    map.entities.push(MapEntity {
        kind: MapEntityKind::Unit,
        owner: "AMERICANS".into(),
        type_id: "MTNK".into(),
        health: 256,
        x: 8,
        y: 4,
        facing: 0,
        sub_cell: 0,
        mission: Default::default(),
        tag: Default::default(),
    });
    map.entities.push(MapEntity {
        kind: MapEntityKind::Infantry,
        owner: "RUSSIANS".into(),
        type_id: "E1".into(),
        health: 256,
        x: 10,
        y: 6,
        facing: 0,
        sub_cell: 1,
        mission: Default::default(),
        tag: Default::default(),
    });
    map.entities.push(MapEntity {
        kind: MapEntityKind::Aircraft,
        owner: "AMERICANS".into(),
        type_id: "ORCA".into(),
        health: 256,
        x: 12,
        y: 8,
        facing: 0,
        sub_cell: 0,
        mission: Default::default(),
        tag: Default::default(),
    });
    // 桥面 overlay：无 TMP 时 finalize 仍会按 Land= 打开该格。
    map.overlays.push(OverlayCell { x: 1, y: 1, overlay_id: 0, data: 0 });
    map
}

struct RulesBytesSource;
impl AssetSource for RulesBytesSource {
    fn read(&self, relative: &str) -> RaResult<Vec<u8>> {
        if relative.eq_ignore_ascii_case(RULES_INI) {
            Ok(b"[General]\n".to_vec())
        }
        else {
            Err(RaError::MissingFile(relative.to_string()))
        }
    }
}

fn open_prepared(map: MapInfo, defs: std::sync::Arc<ra_types::RuntimeDefinitions>) -> ra_engine::SkirmishOpenResult {
    let prepared = validate_map_for_battle(&map, &defs).expect("prepare");
    open_campaign_session_prepared(
        &RulesBytesSource,
        GameEdition::Ra2,
        RULES_INI,
        defs,
        map,
        prepared,
        "map3".into(),
        (0, 0),
        Some("AMERICANS"),
        &["AMERICANS", "RUSSIANS"],
        0,
    )
    .expect("campaign prepared open")
}

/// 会话入口：placement → TechnoId/HouseId → Foundation occupancy → PassGrid → ECS spawn → finalize 回写。
#[test]
fn open_campaign_prepare_chain_foundation_overlay_spawn_and_sync() {
    let defs = prepare_defs();
    let gapowr = defs.techno.get("GAPOWR").expect("GAPOWR").id;
    let orca = defs.techno.get("ORCA").expect("ORCA").id;
    let americans = defs.houses.get("AMERICANS").expect("AMERICANS").id;

    let opened = open_prepared(prepare_map(), defs);
    let state = &opened.session.expect_battle().world;
    assert_eq!(state.prepared.placements.len(), 4);
    assert_eq!(state.entity_count(), 4);
    assert_eq!(state.prepared.houses.len(), 1);
    assert_eq!(state.prepared.houses[0].country, americans);
    assert_eq!(state.prepared.houses[0].credits, 50);

    let structure = state.prepared.placements.iter().find(|p| p.definition_id == gapowr).expect("structure placement");
    assert_eq!(structure.owner, americans);
    assert_eq!((structure.x, structure.y), (4, 4));

    let aircraft = state.prepared.placements.iter().find(|p| p.definition_id == orca).expect("aircraft placement");
    assert_eq!((aircraft.x, aircraft.y), (12, 8));

    let idx = |x: u16, y: u16| (y as usize) * (state.prepared.pass_width as usize) + (x as usize);
    assert_eq!(state.prepared.occupancy[idx(4, 4)], occupancy_kind::STRUCTURE);
    assert_eq!(state.prepared.occupancy[idx(5, 5)], occupancy_kind::STRUCTURE);
    assert_eq!(state.prepared.passable[idx(4, 4)], 0);
    assert_eq!(state.prepared.passable[idx(5, 5)], 0);
    assert!(!state.pass_grid.is_passable(4, 4));
    assert!(!state.pass_grid.is_passable(5, 5));

    // finalize 后 prepared 通行层必须与权威 pass_grid 一致。
    assert_eq!(state.prepared.pass_width, state.pass_grid.width);
    assert_eq!(state.prepared.pass_height, state.pass_grid.height);
    for y in 0..state.prepared.pass_height as u16 {
        for x in 0..state.prepared.pass_width as u16 {
            let prepared_pass = state.prepared.passable[idx(x, y)] != 0;
            assert_eq!(prepared_pass, state.pass_grid.is_passable(x, y), "pass sync mismatch at ({x},{y})");
        }
    }

    // 桥 overlay：空 TMP 源下仍应由 overlay land 保证可走。
    assert!(state.pass_grid.is_passable(1, 1));
    assert_eq!(state.prepared.passable[idx(1, 1)], 1);

    assert_eq!(state.prepared.definition.waypoints.len(), 1);
    assert_eq!(state.prepared.definition.overlays.len(), 1);

    let power_id = state.entity_id_at(0).expect("structure entity");
    let identity = state.ecs_identity(power_id).expect("identity");
    assert_eq!(identity.0.as_ref(), "GAPOWR");
    assert_eq!(identity.1, MapEntityKind::Structure);

    let aircraft_id = state.entity_id_at(3).expect("aircraft entity");
    let aircraft_identity = state.ecs_identity(aircraft_id).expect("aircraft identity");
    assert_eq!(aircraft_identity.0.as_ref(), "ORCA");
    assert_eq!(aircraft_identity.1, MapEntityKind::Aircraft);
}

#[test]
fn validate_map_for_battle_rejects_unknown_map_placement_house() {
    let mut map = prepare_map();
    map.entities[0].owner = "NO_SUCH_HOUSE".into();
    let err = validate_map_for_battle(&map, &prepare_defs()).expect_err("unknown map house must fail prepare");
    let msg = err.to_string();
    assert!(msg.contains("house") || msg.contains("NO_SUCH_HOUSE") || msg.contains("Owner") || msg.contains("owner"), "{msg}");
}

#[test]
fn validate_map_for_battle_rejects_unknown_techno_before_session() {
    let mut map = prepare_map();
    map.entities[1].type_id = "MISSINGUNIT".into();
    let err = validate_map_for_battle(&map, &prepare_defs()).expect_err("unknown techno");
    let msg = err.to_string();
    assert!(msg.contains("techno") || msg.contains("MISSINGUNIT"), "{msg}");
}

#[test]
fn validate_map_for_battle_rejects_out_of_bounds_placement() {
    let mut map = prepare_map();
    map.entities[1].x = 16;
    map.entities[1].y = 0;
    let err = validate_map_for_battle(&map, &prepare_defs()).expect_err("oob placement");
    let msg = err.to_string();
    assert!(msg.contains("越界") || msg.contains("out"), "{msg}");
}

#[test]
fn validate_map_for_battle_rejects_foundation_out_of_bounds() {
    let mut map = prepare_map();
    // 2x2 Foundation 锚在右下角会踩出地图。
    map.entities[0].x = 15;
    map.entities[0].y = 15;
    let err = validate_map_for_battle(&map, &prepare_defs()).expect_err("foundation oob");
    let msg = err.to_string();
    assert!(msg.contains("越界") || msg.contains("Foundation"), "{msg}");
}

#[test]
fn validate_map_for_battle_rejects_overlapping_structures() {
    let mut map = prepare_map();
    map.entities.push(MapEntity {
        kind: MapEntityKind::Structure,
        owner: "AMERICANS".into(),
        type_id: "GAPOWR".into(),
        health: 256,
        x: 5,
        y: 5,
        facing: 0,
        sub_cell: 0,
        mission: Default::default(),
        tag: Default::default(),
    });
    let err = validate_map_for_battle(&map, &prepare_defs()).expect_err("overlap");
    let msg = err.to_string();
    assert!(msg.contains("重叠") || msg.contains("overlap"), "{msg}");
}

#[test]
fn validate_map_for_battle_accepts_prepare_chain_fixture() {
    let prepared = validate_map_for_battle(&prepare_map(), &prepare_defs()).expect("fixture must prepare");
    assert_eq!(prepared.placements.len(), 4);
    assert_eq!(prepared.houses.len(), 1);
    assert!(prepared.placements.iter().any(|p| p.kind == ra_types::MapPlacedEntityKind::Aircraft));
}

#[test]
fn open_campaign_session_prepared_reuses_validated_prepared_map() {
    let defs = prepare_defs();
    let map = prepare_map();
    let prepared = validate_map_for_battle(&map, &defs).expect("prepare");
    let placement_count = prepared.placements.len();
    let opened = open_campaign_session_prepared(
        &RulesBytesSource,
        GameEdition::Ra2,
        RULES_INI,
        defs,
        map,
        prepared,
        "reuse".into(),
        (0, 0),
        Some("AMERICANS"),
        &["AMERICANS", "RUSSIANS"],
        0,
    )
    .expect("campaign prepared open");
    assert!(opened.note.contains("prepared#"), "{}", opened.note);
    let world = &opened.session.expect_battle().world;
    assert_eq!(world.prepared.placements.len(), placement_count);
    assert_eq!(world.entity_count(), placement_count);
}
