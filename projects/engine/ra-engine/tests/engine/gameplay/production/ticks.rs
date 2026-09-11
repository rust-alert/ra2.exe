//! 自 `engine/ra-engine/src/gameplay/production.rs` 迁出的单元测试（集成测试 crate）。

// 自 engine/ra-engine/src/gameplay/production.rs :: tests
use ra_engine::{
    gameplay::production::produce_ticks_for,
    state::battle::{BUILD_TIME_TICKS_PER_UNIT, PRODUCE_TICKS},
};
use ra_types::{TechnoClass, TechnoDefinition, TypeId};

fn sample(build_time: u32) -> TechnoDefinition {
    TechnoDefinition {
        id: TypeId(1),
        type_key: "E1".into(),
        class: TechnoClass::Infantry,
        cost: 200,
        strength: 125,
        armor: ra_types::ArmorKind::None,
        speed: 4,
        owner: ra_types::HouseAllowList::parse_owner("Americans"),
        tech_level: 1,
        naval: false,
        agent: false,
        engineer: false,
        harvester: false,
        category: ra_types::TechnoCategory::Unspecified,
        sight: 5,
        primary: ra_types::WeaponName::default(),
        primary_id: ra_types::WeaponId(0),
        secondary: ra_types::WeaponName::default(),
        secondary_id: ra_types::WeaponId(0),
        warhead: ra_types::WarheadName::default(),
        warhead_id: ra_types::WarheadId(0),
        prerequisite: Vec::new(),
        prerequisite_override: Vec::new(),
        required_houses: ra_types::HouseAllowList::empty(),
        forbidden_houses: ra_types::HouseAllowList::empty(),
        build_limit: 0,
        build_time,
        requires_stolen_allied_tech: false,
        requires_stolen_soviet_tech: false,
        requires_stolen_third_tech: false,
        pixel_selection_bracket_delta: 0,
    }
}

#[test]
fn missing_build_time_falls_back_to_produce_ticks() {
    assert_eq!(produce_ticks_for(&sample(0)), PRODUCE_TICKS);
}

#[test]
fn build_time_scales_to_ticks() {
    assert_eq!(produce_ticks_for(&sample(5)), 5 * BUILD_TIME_TICKS_PER_UNIT);
    assert_eq!(produce_ticks_for(&sample(1)), BUILD_TIME_TICKS_PER_UNIT.max(1));
}
