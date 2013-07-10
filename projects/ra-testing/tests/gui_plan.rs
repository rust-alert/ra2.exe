use std::path::PathBuf;

use ra_testing::standard_duel_gui_plan;

#[test]
fn duel_plan_has_harness_args_and_status_waits() {
    let plan = standard_duel_gui_plan(PathBuf::from("ra2"), PathBuf::from("."), PathBuf::from("status.txt"));
    assert_eq!(plan.name, "standard-duel");
    assert!(plan.args.iter().any(|a| a.contains("test-scene=duel")));
    assert!(plan.actions.iter().any(|a| matches!(a, ra_testing::GuiAction::WaitStatus { .. })));
}
