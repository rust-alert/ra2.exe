//! 动作码 63：航点处 Apply 100 damage。

use crate::common::{battle_from_defs, defs_from_rules_ini, test_engine};
use ra_engine::{Session, SessionBootKind};
use ra_map::{MapActionKind, MapEntity, MapEntityKind, MapInfo, Waypoint, campaign_blocking_capability_message, map_scripting_capability_gaps};
use ra_types::GameEdition;

#[test]
fn apply_100_damage_kills_unit_at_waypoint() {
    let defs = defs_from_rules_ini(
        b"[InfantryTypes]\n0=E1\n\
[E1]\nStrength=80\nSpeed=4\nSight=4\nCost=100\nArmor=none\nOwner=Americans\n",
    );
    let text = b"\
[Map]\nSize=0,0,16,16\nTheater=TEMPERATE\n\
[Houses]\n0=Americans\n\
[Americans]\nCountry=Americans\nPlayerControl=yes\n\
[Waypoints]\n5=5005\n\
[Tags]\nT1=0,Boom,TR1\n\
[Triggers]\nTR1=Americans,<none>,Boom,0,1,1,1,0\n\
[Events]\nTR1=1,13,0,0\n\
[Actions]\nTR1=1,63,0,5,0,0,0,0,A\n\
";
    let mut map = MapInfo::parse_ini(GameEdition::Ra2, "dmg.map", text).unwrap();
    assert_eq!(map.scripting.actions[0].commands[0].kind, MapActionKind::Apply100Damage);
    map.waypoints = vec![Waypoint { index: 5, x: 5, y: 5 }];
    map.entities.push(MapEntity {
        kind: MapEntityKind::Infantry,
        owner: "AMERICANS".into(),
        type_id: "E1".into(),
        health: 256,
        x: 5,
        y: 5,
        facing: 0,
        sub_cell: 0,
        mission: Default::default(),
        tag: Default::default(),
    });

    let engine = test_engine();
    let mut session = Session::from_state(battle_from_defs(GameEdition::Ra2, defs, map), "dmg");
    session.expect_battle_mut().boot_kind = SessionBootKind::Campaign;
    session.expect_battle_mut().world.ensure_house("AMERICANS");
    let id = session.expect_battle().world.entity_id_at(0).expect("unit");
    assert_eq!(session.expect_battle().world.ecs_health(id).map(|h| h.0), Some(80));

    session.tick(&engine.runtime());
    assert!(
        session.expect_battle().world.ecs_health(id).map(|h| h.2).unwrap_or(false),
        "100 damage should kill Strength=80 infantry at waypoint"
    );
}

#[test]
fn apply_100_damage_is_not_campaign_blocking_gap() {
    let text = b"\
[Map]\nSize=0,0,8,8\nTheater=TEMPERATE\n\
[Triggers]\nTR1=Americans,<none>,X,0,1,1,1,0\n\
[Events]\nTR1=1,13,0,0\n\
[Actions]\nTR1=1,63,0,0,0,0,0,0,A\n\
";
    let map = MapInfo::parse_ini(GameEdition::Ra2, "gap.map", text).unwrap();
    let gaps = map_scripting_capability_gaps(&map);
    assert!(gaps.iter().all(|g| g.code != "map.action.63 unsupported"), "{gaps:?}");
    assert!(campaign_blocking_capability_message(&map).is_none());
}
