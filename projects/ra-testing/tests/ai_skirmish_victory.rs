//! 局部胜负 / 确定性夹具：关 AI、改敌方建造场血量后脚本攻击。
//!
//! **证明范围**：命令路径可打到结算，以及同等干预下状态哈希一致。
//! **不证明**：持续 AI 对抗、完整遭遇战、仅靠产品命令的无干预单局。

use ra_engine::{GameCommand, MatchOutcome};
use ra_testing::{ai_skirmish_open, alpha_skirmish_v1};

#[test]
fn scripted_attack_after_ai_deploy_reaches_victory() {
    let slice = alpha_skirmish_v1();
    let mut case = ai_skirmish_open();
    case.advance(1);
    assert!(case
        .session
        .expect_game_mut()
        .world
        .entities
        .iter()
        .any(|e| e.owner.as_ref() == slice.ai_house && e.type_id.as_ref() == "NACNST"));
    // 停止继续扩建，只验收「可经命令路径打到结算」。
    case.session.expect_game_mut().ai_enabled = false;
    let tank = case
        .session
        .expect_game()
        .world
        .entities
        .iter()
        .position(|e| e.owner.as_ref() == slice.human_house && e.type_id.as_ref() == "MTNK")
        .expect("human tank");
    let yard = case
        .session
        .expect_game_mut()
        .world
        .entities
        .iter()
        .position(|e| e.owner.as_ref() == slice.ai_house && e.type_id.as_ref() == "NACNST")
        .expect("ai yard");
    let tank_id = case.session.expect_game().world.entities[tank].id;
    let yard_id = case.session.expect_game().world.entities[yard].id;
    case.session.expect_game_mut().world.entities[yard].health = 120;
    case.command(GameCommand::Attack { attacker: tank_id, target: yard_id });
    case.advance(64);
    let result = case.observe();
    assert_eq!(result.outcome, Some(MatchOutcome::Victory { owner: slice.human_house.into() }));
    let stats = result.snapshot.match_stats.expect("match stats");
    assert!(stats.duration_ticks > 0);
    assert!(stats.buildings_lost >= 1);
    assert!(case.session.expect_game().paused);
}

#[test]
fn equal_scripted_interventions_match_hash() {
    let slice = alpha_skirmish_v1();
    let mut first = ai_skirmish_open();
    let mut second = ai_skirmish_open();
    for case in [&mut first, &mut second] {
        case.advance(1);
        case.session.expect_game_mut().ai_enabled = false;
        let tank = case
            .session
            .expect_game_mut()
            .world
            .entities
            .iter()
            .position(|e| e.owner.as_ref() == slice.human_house && e.type_id.as_ref() == "MTNK")
            .unwrap();
        let yard = case
            .session
            .expect_game_mut()
            .world
            .entities
            .iter()
            .position(|e| e.owner.as_ref() == slice.ai_house && e.type_id.as_ref() == "NACNST")
            .unwrap();
        let tank_id = case.session.expect_game().world.entities[tank].id;
        let yard_id = case.session.expect_game().world.entities[yard].id;
        case.session.expect_game_mut().world.entities[yard].health = 120;
        case.command(GameCommand::Attack { attacker: tank_id, target: yard_id });
        case.advance(32);
    }
    let a = first.observe();
    let b = second.observe();
    assert_eq!(a.tick, b.tick);
    assert_eq!(a.state_hash, b.state_hash);
    assert_eq!(a.outcome, b.outcome);
}
