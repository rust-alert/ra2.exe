//! 快照画面：对局中 / 结算。

mod common;

use common::rules_with_mtnk;
use ra_engine::{GameCommand, MatchOutcome, Session, SessionScreen, World};
use ra_map::{MapEntity, MapEntityKind, MapInfo};
use ra_types::GameEdition;

#[test]
fn snapshot_screen_moves_to_results_on_victory() {
    let rules = rules_with_mtnk();
    let mut map = MapInfo::empty(GameEdition::Ra2, "screen");
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
    let mut session = Session::new(World::new(GameEdition::Ra2, &rules, map), "screen");
    assert_eq!(session.snapshot().screen, SessionScreen::InMatch);
    session.world.entities[0].attack_damage = 80;
    session.world.entities[0].attack_range = 4;
    session.world.entities[0].attack_cooldown_max = 1;
    session.world.entities[1].health = 40;
    session.push_command(GameCommand::Attack { attacker_index: 0, target_index: 1 });
    for _ in 0..20 {
        session.tick();
        if session.outcome.is_some() {
            break;
        }
    }
    assert!(matches!(session.outcome, Some(MatchOutcome::Victory { .. })));
    assert_eq!(session.snapshot().screen, SessionScreen::Results);
}
