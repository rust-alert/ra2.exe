//! 经 Session 的生产队列。

use ra_engine::{GameCommand, PRODUCE_TICKS};
use ra_map::MapEntityKind;
use ra_testing::{alpha_skirmish_v1, yard_open};
use ra_types::PlayerId;

#[test]
fn produce_infantry_through_session_after_barracks() {
    let mut case = yard_open();
    let slice = alpha_skirmish_v1();
    case.command(GameCommand::PlaceBuilding { player: PlayerId(0), type_id: "GAPOWR".into(), x: 6, y: 4 });
    case.advance(1);
    case.command(GameCommand::PlaceBuilding { player: PlayerId(0), type_id: "GAPILE".into(), x: 8, y: 4 });
    case.advance(1);
    let before = case.session.world.house_funds(slice.human_house).expect("应有资金");
    case.command(GameCommand::Produce { player: PlayerId(0), type_id: "E1".into() });
    case.advance(1);
    assert_eq!(case.session.world.house_funds(slice.human_house), Some(before - 200));
    case.advance(u64::from(PRODUCE_TICKS - 1));
    assert_eq!(case.session.world.entities.len(), 4);
    let unit = case.session.world.entities.last().expect("应产出单位");
    assert_eq!(unit.kind, MapEntityKind::Infantry);
    assert_eq!(unit.type_id, "E1");
}
