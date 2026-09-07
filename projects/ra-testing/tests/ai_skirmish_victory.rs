//! AI 遭遇战后脚本攻击至胜负，并校验确定性。

use ra_engine::{GameCommand, MatchOutcome};
use ra_testing::{ai_skirmish_open, alpha_skirmish_v1};

#[test]
fn scripted_attack_after_ai_deploy_reaches_victory() {
    let slice = alpha_skirmish_v1();
    let mut case = ai_skirmish_open();
    case.advance(1);
    assert!(case.session.world.entities.iter().any(|e| e.owner == slice.ai_house && e.type_id == "NACNST"));
    // 停止继续扩建，只验收「可经命令路径打到结算」。
    case.session.ai_enabled = false;
    let tank = case
        .session
        .world
        .entities
        .iter()
        .position(|e| e.owner == slice.human_house && e.type_id == "MTNK")
        .expect("human tank");
    let yard =
        case.session.world.entities.iter().position(|e| e.owner == slice.ai_house && e.type_id == "NACNST").expect("ai yard");
    case.session.world.entities[yard].health = 120;
    case.command(GameCommand::Attack { attacker_index: tank, target_index: yard });
    case.advance(64);
    let result = case.observe();
    assert_eq!(result.outcome, Some(MatchOutcome::Victory { owner: slice.human_house.into() }));
    let stats = result.snapshot.match_stats.expect("match stats");
    assert!(stats.duration_ticks > 0);
    assert!(stats.buildings_lost >= 1);
    assert!(case.session.paused);
}

#[test]
fn equal_ai_skirmish_scripts_match_hash() {
    let slice = alpha_skirmish_v1();
    let mut first = ai_skirmish_open();
    let mut second = ai_skirmish_open();
    for case in [&mut first, &mut second] {
        case.advance(1);
        case.session.ai_enabled = false;
        let tank =
            case.session.world.entities.iter().position(|e| e.owner == slice.human_house && e.type_id == "MTNK").unwrap();
        let yard = case.session.world.entities.iter().position(|e| e.owner == slice.ai_house && e.type_id == "NACNST").unwrap();
        case.session.world.entities[yard].health = 120;
        case.command(GameCommand::Attack { attacker_index: tank, target_index: yard });
        case.advance(32);
    }
    let a = first.observe();
    let b = second.observe();
    assert_eq!(a.tick, b.tick);
    assert_eq!(a.state_hash, b.state_hash);
    assert_eq!(a.outcome, b.outcome);
}
