//! 胜负时锁定 BattleStats。

use crate::common::{rules_with_mtnk, test_engine};
use ra_engine::{GameCommand, BattleOutcome, BattleState, Session};
use ra_map::{MapEntity, MapEntityKind, MapInfo};
use ra_types::{EntityId, GameEdition};

#[test]
fn victory_locks_battle_stats() {
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
        mission: String::new(),
        tag: String::new(),
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
        mission: String::new(),
        tag: String::new(),
    });
    let mut session = Session::from_state(BattleState::new(GameEdition::Ra2, &rules, map), "stats");
    let attacker = session.expect_battle().world.entity_id_at(0).expect("entity");
    let target = session.expect_battle().world.entity_id_at(1).expect("entity");
    assert!(session.expect_battle_mut().world.set_ecs_attack_power(attacker, 80, 4, 1));
    assert!(session.expect_battle_mut().world.set_ecs_health(target, 50, 50, false));
    session.expect_battle_mut().world.players[0].funds_spent = 1200;
    session.expect_battle_mut().push_command(GameCommand::Attack { attacker: EntityId(1), target: EntityId(2) });
    for _ in 0..20 {
        session.tick(&engine.runtime());
        if session.expect_battle().outcome.is_some() {
            break;
        }
    }
    assert_eq!(session.expect_battle().outcome, Some(BattleOutcome::Victory { owner: "Americans".into() }));
    let stats = session.expect_battle().battle_stats.as_ref().expect("stats");
    assert!(stats.duration_ticks > 0);
    assert_eq!(stats.units_lost, 1);
    assert_eq!(stats.buildings_lost, 0);
    assert_eq!(stats.funds_spent, 1200);
    let americans = stats.players.iter().find(|p| p.house == "Americans").expect("Americans row");
    assert_eq!(americans.kills, 1);
    assert_eq!(americans.losses, 0);
    assert_eq!(session.expect_battle().snapshot(&[]).battle_stats.as_ref().map(|s| s.funds_spent), Some(1200));
}
