//! Map-3：会话装载走 `to_prepared_map` → spawn → `finalize_pass_from_assets` 闭环。

use crate::common::defs_from_rules_ini;
use ra_engine::open_campaign_session;
use ra_map::{MapEntity, MapEntityKind, MapInfo, OverlayCell, Waypoint};
use ra_types::{AssetSource, GameEdition, RaError, RaResult, occupancy_kind};

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
[BuildingTypes]\n0=GAPOWR\n\
[E1]\nOwner=Americans\nStrength=125\nSpeed=24\nSight=4\nCost=200\n\
[MTNK]\nOwner=Americans\nStrength=400\nSpeed=64\nSight=6\nCost=800\n\
[GAPOWR]\nPower=200\nOwner=Americans\nStrength=600\nSight=4\nCost=600\nTechLevel=1\nFoundation=2x2\n",
    )
}

fn prepare_map() -> MapInfo {
    let mut map = MapInfo::empty(GameEdition::Ra2, "map3-session");
    map.width = 16;
    map.height = 16;
    map.waypoints = vec![Waypoint { index: 0, x: 2, y: 2 }];
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

/// 会话入口：placement → TechnoId/HouseId → Foundation occupancy → PassGrid → ECS spawn → finalize 回写。
#[test]
fn open_campaign_prepare_chain_foundation_overlay_spawn_and_sync() {
    let defs = prepare_defs();
    let gapowr = defs.techno.get("GAPOWR").expect("GAPOWR").id;
    let americans = defs.houses.get("AMERICANS").expect("AMERICANS").id;

    let opened = open_campaign_session(
        &RulesBytesSource,
        GameEdition::Ra2,
        RULES_INI,
        defs,
        prepare_map(),
        "map3".into(),
        (0, 0),
        Some("AMERICANS"),
        &["AMERICANS", "RUSSIANS"],
        0,
    )
    .expect("campaign open");

    let state = &opened.session.expect_battle().world;
    assert_eq!(state.prepared.placements.len(), 3);
    assert_eq!(state.entity_count(), 3);

    let structure = state.prepared.placements.iter().find(|p| p.definition_id == gapowr).expect("structure placement");
    assert_eq!(structure.owner, americans);
    assert_eq!((structure.x, structure.y), (4, 4));

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
}

#[test]
fn open_campaign_rejects_unknown_map_placement_house() {
    let mut map = prepare_map();
    map.entities[0].owner = "NO_SUCH_HOUSE".into();
    let err = open_campaign_session(
        &RulesBytesSource,
        GameEdition::Ra2,
        RULES_INI,
        prepare_defs(),
        map,
        "t".into(),
        (0, 0),
        Some("AMERICANS"),
        &["AMERICANS"],
        0,
    )
    .expect_err("unknown map house must fail open");
    let msg = err.to_string();
    assert!(msg.contains("house") || msg.contains("NO_SUCH_HOUSE") || msg.contains("Owner") || msg.contains("owner"), "{msg}");
}
