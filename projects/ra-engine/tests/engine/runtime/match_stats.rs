//! 胜负时锁定 MatchStats。

use crate::common::rules_with_mtnk;
use ra_engine::{GameCommand, MatchOutcome, Session, World};
use ra_map::{MapEntity, MapEntityKind, MapInfo};
use ra_types::GameEdition;

#[test]
fn victory_locks_match_stats() {
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
    let mut session = Session::new(World::new(GameEdition::Ra2, &rules, map), "stats");
    session.world.entities[0].attack_damage = 80;
    session.world.entities[0].attack_range = 4;
    session.world.entities[0].attack_cooldown_max = 1;
    session.world.entities[1].health = 50;
    session.world.players[0].funds_spent = 1200;
    session.push_command(GameCommand::Attack { attacker_index: 0, target_index: 1 });
    for _ in 0..20 {
        session.tick();
        if session.outcome.is_some() {
            break;
        }
    }
    assert_eq!(session.outcome, Some(MatchOutcome::Victory { owner: "Americans".into() }));
    let stats = session.match_stats.as_ref().expect("stats");
    assert!(stats.duration_ticks > 0);
    assert_eq!(stats.units_lost, 1);
    assert_eq!(stats.buildings_lost, 0);
    assert_eq!(stats.funds_spent, 1200);
    assert_eq!(session.snapshot().match_stats.as_ref().map(|s| s.funds_spent), Some(1200));
}
