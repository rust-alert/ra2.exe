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
        armor: "none".into(),
        speed: 4,
        owner: "Americans".into(),
        tech_level: 1,
        naval: false,
        agent: false,
        engineer: false,
        harvester: false,
        category: String::new(),
        sight: 5,
        damage: 0,
        range: 0,
        rof: 0,
        warhead: String::new(),
        warhead_id: TypeId(0),
        prerequisite: Vec::new(),
        prerequisite_override: Vec::new(),
        required_houses: Vec::new(),
        forbidden_houses: Vec::new(),
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
