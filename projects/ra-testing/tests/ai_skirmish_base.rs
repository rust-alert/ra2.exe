//! AI 遭遇战：经同一命令路径部署并供电。

use ra_map::MapEntityKind;
use ra_testing::{ai_skirmish_open, alpha_skirmish_v1};

#[test]
fn ai_deploys_mcv_and_places_power() {
    let slice = alpha_skirmish_v1();
    let mut case = ai_skirmish_open();
    assert!(case.session.expect_game_mut().ai_enabled);
    case.advance(1);
    let yard = case
        .session
        .expect_game_mut()
        .world
        .entities
        .iter()
        .find(|e| e.owner.as_ref() == slice.ai_house && e.type_id.as_ref() == "NACNST");
    assert!(yard.is_some(), "AI should deploy SMCV into NACNST");
    assert_eq!(yard.unwrap().kind, MapEntityKind::Structure);

    case.advance(1);
    let power = case
        .session
        .expect_game_mut()
        .world
        .entities
        .iter()
        .find(|e| e.owner.as_ref() == slice.ai_house && e.type_id.as_ref() == "NAPOWR");
    assert!(power.is_some(), "AI should place NAPOWR near yard");
    assert!(case.observe().outcome.is_none());
}
