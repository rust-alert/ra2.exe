//! 自顶层 `headless_duel.rs`。

use ra_engine::{BattleOutcome, GameCommand};
use ra_testing::standard_duel;
use ra_types::EntityId;

#[test]
fn standard_duel_reaches_a_repeatable_victory() {
    let mut case = standard_duel();
    case.command(GameCommand::Attack { attacker: EntityId(1), target: EntityId(2) });
    case.advance(64);
    let result = case.observe();
    assert_eq!(result.outcome, Some(BattleOutcome::Victory { owner: "AMERICANS".into() }));
    assert!(result.tick > 0);
    assert!(result.snapshot.units.iter().any(|unit| unit.dead));
}

#[test]
fn equal_scripts_produce_equal_observations() {
    let mut first = standard_duel();
    let mut second = standard_duel();
    for case in [&mut first, &mut second] {
        case.command(GameCommand::MoveTo { entity: EntityId(1), x: 5, y: 8 });
        case.advance(3);
    }
    let a = first.observe();
    let b = second.observe();
    assert_eq!(a.tick, b.tick);
    assert_eq!(a.state_hash, b.state_hash);
    assert_eq!(a.snapshot.units[0].x, b.snapshot.units[0].x);
}

#[test]
fn move_script_advances_unit_cell() {
    let mut case = standard_duel();
    case.command(GameCommand::MoveTo { entity: EntityId(1), x: 6, y: 8 });
    case.advance(4);
    let result = case.observe();
    assert_eq!(result.snapshot.units[0].x, 6);
    assert_eq!(result.snapshot.units[0].y, 8);
    assert!(result.outcome.is_none());
}

#[test]
fn out_of_bounds_commands_are_ignored_without_panic() {
    let mut case = standard_duel();
    case.command(GameCommand::MoveTo { entity: EntityId(99), x: 1, y: 1 });
    case.command(GameCommand::Attack { attacker: EntityId(1), target: EntityId(99) });
    case.advance(2);
    let result = case.observe();
    assert_eq!(result.snapshot.units[0].x, 4);
    assert!(!result.snapshot.units[1].dead);
    assert!(result.outcome.is_none());
}

#[test]
fn victory_pauses_further_ticks() {
    let mut case = standard_duel();
    case.command(GameCommand::Attack { attacker: EntityId(1), target: EntityId(2) });
    case.advance(64);
    let after_win = case.observe();
    assert!(after_win.outcome.is_some());
    let tick_at_win = after_win.tick;
    case.advance(8);
    assert_eq!(case.observe().tick, tick_at_win);
    assert!(case.session.expect_battle().paused);
}
