//! 自 `engine/ra-engine/src/game/capabilities.rs` 迁出的单元测试（集成测试 crate）。

// 自 engine/ra-engine/src/game/capabilities.rs :: tests
use ra_engine::{CommandRejectReason, game::capabilities::*};

#[test]
fn losing_construction_yard_disables_all_build_tech() {
    let (ok, reason) = evaluate_build_availability(true, true, false, false);
    assert!(ok);
    assert_eq!(reason, None);

    let (ok, reason) = evaluate_build_availability(false, true, false, false);
    assert!(!ok);
    assert_eq!(reason, Some(CommandRejectReason::MissingPrerequisite));
}

#[test]
fn losing_power_plant_blocks_power_gated_buildings_only() {
    let (ok, _) = evaluate_build_availability(true, false, false, false);
    assert!(ok);
    let (ok, reason) = evaluate_build_availability(true, false, true, false);
    assert!(!ok);
    assert_eq!(reason, Some(CommandRejectReason::InsufficientPower));
}

#[test]
fn losing_factory_disables_produce_tech() {
    let (ok, reason) = evaluate_produce_availability(false, true, false);
    assert!(!ok);
    assert_eq!(reason, Some(CommandRejectReason::MissingPrerequisite));

    let (ok, reason) = evaluate_produce_availability(true, false, false);
    assert!(!ok);
    assert_eq!(reason, Some(CommandRejectReason::QueueFull));
}

#[test]
fn low_funds_do_not_disable_build_or_produce_cameos() {
    let (ok, reason) = evaluate_build_availability(true, true, false, false);
    assert!(ok);
    assert_eq!(reason, None);
    let (ok, reason) = evaluate_produce_availability(true, true, false);
    assert!(ok);
    assert_eq!(reason, None);
}
