//! 胜负时锁定 MatchStats。

use crate::common::{rules_with_mtnk, test_engine};
use ra_engine::{GameCommand, MatchOutcome, MatchState, Session};
use ra_map::{MapEntity, MapEntityKind, MapInfo};
use ra_types::{EntityId, GameEdition};

#[test]
fn victory_locks_match_stats() {
    let engine = test_engine();
    let rules = rules_with_mtnk();
    let mut map = MapInfo::empty(GameEdition::Ra2, "stats");
    map.width = 16;
    map.height = 16;
    map.entities.push(MapEntity {
        kind: MapEntityKind::Unit,
        owner: "Americans".into(),
        type_id: "MTNK".into(),
        health: 256,
        x: 4,
        y: 4,
        facing: 0,
        sub_cell: 0,
    });
    map.entities.push(MapEntity {
        kind: MapEntityKind::Unit,
        owner: "Soviets".into(),
        type_id: "MTNK".into(),
        health: 256,
        x: 5,
        y: 4,
        facing: 0,
        sub_cell: 0,
    });
    let mut session = Session::from_state(MatchState::new(GameEdition::Ra2, &rules, map), "stats");
    let attacker = session.expect_game().world.entities[0].id;
    let target = session.expect_game().world.entities[1].id;
    assert!(session.expect_game_mut().world.set_ecs_attack_power(attacker, 80, 4, 1));
    assert!(session.expect_game_mut().world.set_ecs_health(target, 50, 50, false));
    session.expect_game_mut().world.players[0].funds_spent = 1200;
    session.expect_game_mut().push_command(GameCommand::Attack { attacker: EntityId(1), target: EntityId(2) });
    for _ in 0..20 {
        session.tick(&engine.runtime());
        if session.expect_game().outcome.is_some() {
            break;
        }
    }
    assert_eq!(session.expect_game().outcome, Some(MatchOutcome::Victory { owner: "Americans".into() }));
    let stats = session.expect_game().match_stats.as_ref().expect("stats");
    assert!(stats.duration_ticks > 0);
    assert_eq!(stats.units_lost, 1);
    assert_eq!(stats.buildings_lost, 0);
    assert_eq!(stats.funds_spent, 1200);
    assert_eq!(session.expect_game().snapshot(&[]).match_stats.as_ref().map(|s| s.funds_spent), Some(1200));
}
