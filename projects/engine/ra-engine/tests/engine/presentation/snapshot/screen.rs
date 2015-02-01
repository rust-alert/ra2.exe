//! 快照画面：对局中 / 结算。

use crate::common::{rules_with_mtnk, test_engine};
use ra_engine::{GameCommand, BattleOutcome, BattleState, Session, SessionScreen};
use ra_map::{MapEntity, MapEntityKind, MapInfo};
use ra_types::{EntityId, GameEdition};

#[test]
fn snapshot_screen_moves_to_results_on_victory() {
    let engine = test_engine();
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
    let mut session = Session::from_state(BattleState::new(GameEdition::Ra2, &rules, map), "screen");
    assert_eq!(session.expect_battle().snapshot(&[]).screen, SessionScreen::InBattle);
    let attacker = session.expect_battle().world.entity_id_at(0).expect("entity");
    let target = session.expect_battle().world.entity_id_at(1).expect("entity");
    assert!(session.expect_battle_mut().world.set_ecs_attack_power(attacker, 80, 4, 1));
    assert!(session.expect_battle_mut().world.set_ecs_health(target, 40, 40, false));
    session.expect_battle_mut().push_command(GameCommand::Attack { attacker: EntityId(1), target: EntityId(2) });
    for _ in 0..20 {
        session.tick(&engine.runtime());
        if session.expect_battle().outcome.is_some() {
            break;
        }
    }
    assert!(matches!(session.expect_battle().outcome, Some(BattleOutcome::Victory { .. })));
    assert_eq!(session.expect_battle().snapshot(&[]).screen, SessionScreen::Results);
}
