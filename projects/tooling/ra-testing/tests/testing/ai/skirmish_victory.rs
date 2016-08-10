//! 局部胜负 / 确定性夹具：关 AI、改敌方建造场血量后脚本攻击。
//!
//! **证明范围**：命令路径可打到结算，以及同等干预下状态哈希一致。
//! **不证明**：持续 AI 对抗、完整遭遇战、仅靠产品命令的无干预单局。

use ra_engine::{BattleOutcome, GameCommand};
use ra_testing::{ai_skirmish_open, alpha_skirmish_v1};

#[test]
fn scripted_attack_after_ai_deploy_reaches_victory() {
    let slice = alpha_skirmish_v1();
    let mut case = ai_skirmish_open();
    case.advance(1);
    assert!(case.session.expect_battle().world.find_entity_id_by_owner_type(slice.ai_house, "NACNST").is_some());
    // 停止继续扩建，只验收「可经命令路径打到结算」。
    case.session.expect_battle_mut().ai_enabled = false;
    let tank_id = case.session.expect_battle().world.find_entity_id_by_owner_type(slice.human_house, "MTNK").expect("human tank");
    let yard_id = case.session.expect_battle().world.find_entity_id_by_owner_type(slice.ai_house, "NACNST").expect("ai yard");
    assert!(case.session.expect_battle_mut().world.set_ecs_health(yard_id, 120, 120, false));
    case.command(GameCommand::Attack { attacker: tank_id, target: yard_id });
    case.advance(64);
    let result = case.observe();
    assert_eq!(result.outcome, Some(BattleOutcome::Victory { owner: slice.human_house.into() }));
    let stats = result.snapshot.battle_stats.expect("battle stats");
    assert!(stats.duration_ticks > 0);
    assert!(stats.buildings_lost >= 1);
    assert!(case.session.expect_battle().paused);
}

#[test]
fn equal_scripted_interventions_match_hash() {
    let slice = alpha_skirmish_v1();
    let mut first = ai_skirmish_open();
    let mut second = ai_skirmish_open();
    for case in [&mut first, &mut second] {
        case.advance(1);
        case.session.expect_battle_mut().ai_enabled = false;
        let tank_id = case.session.expect_battle().world.find_entity_id_by_owner_type(slice.human_house, "MTNK").expect("human tank");
        let yard_id = case.session.expect_battle().world.find_entity_id_by_owner_type(slice.ai_house, "NACNST").expect("ai yard");
        assert!(case.session.expect_battle_mut().world.set_ecs_health(yard_id, 120, 120, false));
        case.command(GameCommand::Attack { attacker: tank_id, target: yard_id });
        case.advance(32);
    }
    let a = first.observe();
    let b = second.observe();
    assert_eq!(a.tick, b.tick);
    assert_eq!(a.state_hash, b.state_hash);
    assert_eq!(a.outcome, b.outcome);
}
