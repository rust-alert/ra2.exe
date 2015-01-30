//! AI 遭遇战：经同一命令路径部署并供电。

use ra_map::MapEntityKind;
use ra_testing::{ai_skirmish_open, alpha_skirmish_v1};

#[test]
fn ai_deploys_mcv_and_places_power() {
    let slice = alpha_skirmish_v1();
    let mut case = ai_skirmish_open();
    assert!(case.session.expect_game_mut().ai_enabled);
    case.advance(1);
    let yard_id = case
        .session
        .expect_game()
        .world
        .find_entity_id_by_owner_type(slice.ai_house, "NACNST")
        .expect("AI should deploy SMCV into NACNST");
    let (type_id, kind) = case.session.expect_game().world.ecs_identity(yard_id).expect("yard identity");
    assert_eq!(type_id.as_ref(), "NACNST");
    assert_eq!(kind, MapEntityKind::Structure);

    case.advance(1);
    let power_id = case
        .session
        .expect_game()
        .world
        .find_entity_id_by_owner_type(slice.ai_house, "NAPOWR")
        .expect("AI should place NAPOWR near yard");
    assert!(case.session.expect_game().world.has_ecs_entity(power_id));
    assert!(case.observe().outcome.is_none());
}
