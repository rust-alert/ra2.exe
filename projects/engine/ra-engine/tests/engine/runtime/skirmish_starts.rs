//! 遭遇战开局：席位航点放置 MCV。

use crate::common::defs_from_rules_ini;
use ra_engine::{open_skirmish_session, open_skirmish_session_prepared, strip_skirmish_map_mobiles, validate_map_for_battle};
use ra_map::{MapEntity, MapEntityKind, MapInfo, Waypoint};
use ra_types::{AssetSource, GameEdition, RaError, RaResult};

/// 引擎测试夹具用的 RA2 规则逻辑名（不经 adaptor `ResourceChain`）。
const RULES_INI: &str = "rules.ini";

fn mcv_defs() -> std::sync::Arc<ra_types::RuntimeDefinitions> {
    defs_from_rules_ini(
        b"[VehicleTypes]\n0=AMCV\n1=SMCV\n\
[BuildingTypes]\n0=GACNST\n1=NACNST\n\
[AMCV]\nDeploysInto=GACNST\nOwner=Americans\nStrength=1000\nSpeed=32\nSight=4\nCost=2500\n\
[SMCV]\nDeploysInto=NACNST\nOwner=Russians\nStrength=1000\nSpeed=32\nSight=4\nCost=2500\n\
[GACNST]\nConstructionYard=yes\nOwner=Americans\nStrength=1000\nSight=8\nCost=2500\n\
[NACNST]\nConstructionYard=yes\nOwner=Russians\nStrength=1000\nSight=8\nCost=2500\n",
    )
}

fn map_with_starts() -> MapInfo {
    let mut map = MapInfo::empty(GameEdition::Ra2, "starts");
    map.width = 32;
    map.height = 32;
    map.waypoints = vec![Waypoint { index: 0, x: 4, y: 4 }, Waypoint { index: 1, x: 20, y: 20 }];
    map
}

struct RulesBytesSource;
impl AssetSource for RulesBytesSource {
    fn read(&self, relative: &str) -> RaResult<Vec<u8>> {
        if relative.eq_ignore_ascii_case(RULES_INI) { Ok(b"[General]\n".to_vec()) } else { Err(RaError::MissingFile(relative.to_string())) }
    }
}

#[test]
fn open_skirmish_places_mcv_at_seat_waypoints() {
    let opened = open_skirmish_session(
        &RulesBytesSource,
        GameEdition::Ra2,
        RULES_INI,
        mcv_defs(),
        map_with_starts(),
        "t".into(),
        (0, 0),
        Some("AMERICANS"),
        &["AMERICANS", "RUSSIANS"],
        0,
    )
    .expect("应成功开局");
    let snap = opened.session.expect_battle().snapshot(&[]);
    let units: Vec<_> = snap
        .units
        .iter()
        .filter(|u| matches!(u.type_id.as_ref(), "AMCV" | "SMCV"))
        .map(|u| (u.owner.as_ref().to_string(), u.type_id.as_ref().to_string(), u.x, u.y))
        .collect();
    assert!(units.contains(&("AMERICANS".into(), "AMCV".into(), 4, 4)), "{units:?}");
    assert!(units.contains(&("RUSSIANS".into(), "SMCV".into(), 20, 20)), "{units:?}");
}

#[test]
fn open_skirmish_strips_map_preplaced_mobiles() {
    let mut map = map_with_starts();
    map.entities.push(MapEntity {
        kind: MapEntityKind::Structure,
        owner: "NEUTRAL".into(),
        type_id: "GACNST".into(),
        health: 256,
        x: 8,
        y: 8,
        facing: 0,
        sub_cell: 0,
        mission: Default::default(),
        tag: Default::default(),
    });
    map.entities.push(MapEntity {
        kind: MapEntityKind::Infantry,
        owner: "RUSSIANS".into(),
        type_id: "ADOG".into(),
        health: 256,
        x: 10,
        y: 10,
        facing: 0,
        sub_cell: 0,
        mission: Default::default(),
        tag: Default::default(),
    });
    map.entities.push(MapEntity {
        kind: MapEntityKind::Unit,
        owner: "RUSSIANS".into(),
        type_id: "DEST".into(),
        health: 256,
        x: 12,
        y: 12,
        facing: 0,
        sub_cell: 0,
        mission: Default::default(),
        tag: Default::default(),
    });
    let opened = open_skirmish_session(
        &RulesBytesSource,
        GameEdition::Ra2,
        RULES_INI,
        mcv_defs(),
        map,
        "t".into(),
        (0, 0),
        Some("AMERICANS"),
        &["AMERICANS", "RUSSIANS"],
        0,
    )
    .expect("应成功开局");
    assert!(opened.note.contains("strip_mobiles#2"), "{}", opened.note);
    let snap = opened.session.expect_battle().snapshot(&[]);
    let type_ids: Vec<_> = snap.units.iter().map(|u| u.type_id.as_ref().to_string()).collect();
    assert!(type_ids.iter().any(|t| t == "AMCV"), "{type_ids:?}");
    assert!(type_ids.iter().any(|t| t == "SMCV"), "{type_ids:?}");
    assert!(!type_ids.iter().any(|t| t == "ADOG" || t == "DEST"), "{type_ids:?}");
}

#[test]
fn open_skirmish_fails_when_start_waypoint_missing() {
    let mut map = map_with_starts();
    map.waypoints.retain(|w| w.index == 0);
    let err = open_skirmish_session(
        &RulesBytesSource,
        GameEdition::Ra2,
        RULES_INI,
        mcv_defs(),
        map,
        "t".into(),
        (0, 0),
        Some("AMERICANS"),
        &["AMERICANS", "RUSSIANS"],
        0,
    )
    .unwrap_err();
    let msg = err.to_string();
    assert!(msg.contains("航点") || msg.contains("席位"), "{msg}");
}

#[test]
fn open_skirmish_prepared_reuses_stripped_placements() {
    let mut map = map_with_starts();
    map.entities.push(MapEntity {
        kind: MapEntityKind::Structure,
        owner: "AMERICANS".into(),
        type_id: "GACNST".into(),
        health: 256,
        x: 8,
        y: 8,
        facing: 0,
        sub_cell: 0,
        mission: Default::default(),
        tag: Default::default(),
    });
    map.entities.push(MapEntity {
        kind: MapEntityKind::Unit,
        owner: "AMERICANS".into(),
        type_id: "AMCV".into(),
        health: 256,
        x: 10,
        y: 10,
        facing: 0,
        sub_cell: 0,
        mission: Default::default(),
        tag: Default::default(),
    });
    let defs = mcv_defs();
    let stripped = strip_skirmish_map_mobiles(&mut map);
    assert_eq!(stripped, 1);
    let prepared = validate_map_for_battle(&map, defs.as_ref()).expect("stripped map prepare");
    assert_eq!(prepared.placements.len(), 1, "only structure remains after strip");

    let opened = open_skirmish_session_prepared(
        &RulesBytesSource,
        GameEdition::Ra2,
        RULES_INI,
        defs,
        map,
        prepared,
        "t".into(),
        (0, 0),
        Some("AMERICANS"),
        &["AMERICANS", "RUSSIANS"],
        0,
    )
    .expect("prepared skirmish open");
    assert!(opened.note.contains("prepared#1"), "{}", opened.note);
    let world = &opened.session.expect_battle().world;
    assert_eq!(world.prepared.placements.len(), 1);
    let snap = opened.session.expect_battle().snapshot(&[]);
    let type_ids: Vec<_> = snap.units.iter().map(|u| u.type_id.as_ref().to_string()).collect();
    assert!(type_ids.iter().any(|t| t == "GACNST"), "{type_ids:?}");
    assert!(type_ids.iter().any(|t| t == "AMCV"), "{type_ids:?}");
    assert!(type_ids.iter().any(|t| t == "SMCV"), "{type_ids:?}");
}
