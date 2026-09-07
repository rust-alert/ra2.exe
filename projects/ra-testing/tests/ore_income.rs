//! 经 Session 的矿场收入。

use ra_testing::{alpha_skirmish_v1, yard_open};
use ra_types::PlayerId;
use ra_engine::{GameCommand, ORE_TRIP_TICKS};

#[test]
fn place_refinery_then_ore_trip_credits_through_session() {
    let mut case = yard_open();
    let slice = alpha_skirmish_v1();
    case.command(GameCommand::PlaceBuilding {
        player: PlayerId(0),
        type_id: "GAPOWR".into(),
        x: 6,
        y: 4,
    });
    case.advance(1);
    case.command(GameCommand::PlaceBuilding {
        player: PlayerId(0),
        type_id: "GAREFN".into(),
        x: 8,
        y: 4,
    });
    case.advance(1);
    let after_build = case.session.world.house_funds(slice.human_house).expect("应有资金");
    assert_eq!(after_build, slice.starting_funds - 600 - 2000);
    case.advance(u64::from(ORE_TRIP_TICKS));
    assert_eq!(
        case.session.world.house_funds(slice.human_house),
        Some(after_build + slice.ore_income_per_trip)
    );
}
