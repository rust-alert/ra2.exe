//! 动作码 108：Create Crate（航点刷箱，踩格领固定资金竖切）。

use crate::common::{battle_from_defs, defs_from_rules_ini, test_engine};
use ra_engine::{SCRIPT_CRATE_CREDITS, Session, SessionBootKind};
use ra_map::{MapActionKind, MapEntity, MapEntityKind, MapInfo, Waypoint, campaign_blocking_capability_message, map_scripting_capability_gaps};
use ra_types::GameEdition;

#[test]
fn create_crate_spawns_at_waypoint_when_cell_empty() {
    let defs = defs_from_rules_ini(
        b"[InfantryTypes]\n0=E1\n\
[E1]\nStrength=100\nSpeed=4\nSight=4\nCost=100\nArmor=none\nOwner=Americans\n",
    );
    let text = b"\
[Map]\nSize=0,0,16,16\nTheater=TEMPERATE\n\
[Houses]\n0=Americans\n\
[Americans]\nCountry=Americans\nPlayerControl=yes\n\
[Waypoints]\n3=4004\n\
[Triggers]\nTR1=Americans,<none>,Crate,0,1,1,1,0\n\
[Events]\nTR1=1,13,0,0\n\
[Actions]\nTR1=1,108,0,1,0,0,0,0,3\n\
";
    let mut map = MapInfo::parse_ini(GameEdition::Ra2, "crate.map", text).unwrap();
    assert_eq!(map.scripting.actions[0].commands[0].kind, MapActionKind::CreateCrate);
    map.waypoints = vec![Waypoint { index: 3, x: 4, y: 4 }];
    map.entities.push(MapEntity {
        kind: MapEntityKind::Infantry,
        owner: "AMERICANS".into(),
        type_id: "E1".into(),
        health: 256,
        x: 2,
        y: 2,
        facing: 0,
        sub_cell: 0,
        mission: Default::default(),
        tag: Default::default(),
    });

    let engine = test_engine();
    let mut session = Session::from_state(battle_from_defs(GameEdition::Ra2, defs, map), "crate");
    session.expect_battle_mut().boot_kind = SessionBootKind::Campaign;
    session.expect_battle_mut().world.ensure_house("AMERICANS");
    assert!(session.expect_battle_mut().world.set_house_funds("AMERICANS", 1_000));

    session.tick(&engine.runtime());
    assert_eq!(session.expect_battle().world.trigger_runtime.script_crates.len(), 1);
    assert_eq!(session.expect_battle().world.trigger_runtime.script_crates[0].x, 4);
    assert_eq!(session.expect_battle().world.trigger_runtime.script_crates[0].y, 4);
    assert_eq!(session.expect_battle().world.trigger_runtime.script_crates[0].crate_type, "1");
    assert_eq!(session.expect_battle().world.house_funds("AMERICANS"), Some(1_000));
}

#[test]
fn create_crate_pickup_on_occupied_cell_grants_fixed_credits() {
    let defs = defs_from_rules_ini(
        b"[InfantryTypes]\n0=E1\n\
[E1]\nStrength=100\nSpeed=4\nSight=4\nCost=100\nArmor=none\nOwner=Americans\n",
    );
    let text = b"\
[Map]\nSize=0,0,16,16\nTheater=TEMPERATE\n\
[Houses]\n0=Americans\n\
[Americans]\nCountry=Americans\nPlayerControl=yes\n\
[Waypoints]\n3=4004\n\
[Triggers]\nTR1=Americans,<none>,Crate,0,1,1,1,0\n\
[Events]\nTR1=1,13,0,0\n\
[Actions]\nTR1=1,108,0,1,0,0,0,0,3\n\
";
    let mut map = MapInfo::parse_ini(GameEdition::Ra2, "crate-pick.map", text).unwrap();
    map.waypoints = vec![Waypoint { index: 3, x: 4, y: 4 }];
    map.entities.push(MapEntity {
        kind: MapEntityKind::Infantry,
        owner: "AMERICANS".into(),
        type_id: "E1".into(),
        health: 256,
        x: 4,
        y: 4,
        facing: 0,
        sub_cell: 0,
        mission: Default::default(),
        tag: Default::default(),
    });

    let engine = test_engine();
    let mut session = Session::from_state(battle_from_defs(GameEdition::Ra2, defs, map), "crate-pick");
    session.expect_battle_mut().boot_kind = SessionBootKind::Campaign;
    session.expect_battle_mut().world.ensure_house("AMERICANS");
    assert!(session.expect_battle_mut().world.set_house_funds("AMERICANS", 500));

    session.tick(&engine.runtime());
    assert!(session.expect_battle().world.trigger_runtime.script_crates.is_empty());
    assert_eq!(session.expect_battle().world.house_funds("AMERICANS"), Some(500 + SCRIPT_CRATE_CREDITS));
}

#[test]
fn create_crate_missing_waypoint_records_unsupported() {
    let defs = defs_from_rules_ini(b"[BuildingTypes]\n0=GACNST\n[GACNST]\nConstructionYard=yes\nOwner=Americans\nStrength=1000\n");
    let text = b"\
[Map]\nSize=0,0,8,8\nTheater=TEMPERATE\n\
[Triggers]\nTR1=Americans,<none>,Crate,0,1,1,1,0\n\
[Events]\nTR1=1,13,0,0\n\
[Actions]\nTR1=1,108,0,1,0,0,0,0,A\n\
";
    let map = MapInfo::parse_ini(GameEdition::Ra2, "crate-bad.map", text).unwrap();
    let engine = test_engine();
    let mut session = Session::from_state(battle_from_defs(GameEdition::Ra2, defs, map), "crate-bad");
    session.expect_battle_mut().boot_kind = SessionBootKind::Campaign;
    session.tick(&engine.runtime());
    assert!(session.expect_battle().world.trigger_runtime.unsupported_actions.contains(&108), "missing waypoint must record unsupported 108");
}

#[test]
fn create_crate_is_not_campaign_blocking_gap() {
    let text = b"\
[Map]\nSize=0,0,8,8\nTheater=TEMPERATE\n\
[Triggers]\nTR1=Americans,<none>,X,0,1,1,1,0\n\
[Events]\nTR1=1,13,0,0\n\
[Actions]\nTR1=1,108,0,1,0,0,0,0,3\n\
";
    let map = MapInfo::parse_ini(GameEdition::Ra2, "gap.map", text).unwrap();
    let gaps = map_scripting_capability_gaps(&map);
    assert!(gaps.iter().all(|g| g.code != "map.action.108 unsupported"), "{gaps:?}");
    assert!(campaign_blocking_capability_message(&map).is_none());
}
