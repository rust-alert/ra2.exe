use std::path::PathBuf;

use ra_testing::{
    GuiAction, pre_alpha_acceptance_capture_names, pre_alpha_acceptance_capture_plan, standard_duel_gui_plan,
};

#[test]
fn duel_plan_has_harness_args_and_status_waits() {
    let plan = standard_duel_gui_plan(PathBuf::from("ra2"), PathBuf::from("."), PathBuf::from("status.txt"));
    assert_eq!(plan.name, "standard-duel");
    assert!(plan.args.iter().any(|a| a.contains("test-scene=duel")));
    assert!(plan.actions.iter().any(|a| matches!(a, ra_testing::GuiAction::WaitStatus { .. })));
}

#[test]
fn pre_alpha_acceptance_names_include_lobby_and_results_variants() {
    let names = pre_alpha_acceptance_capture_names();
    assert!(names.contains(&"skirmish_lobby_alt"));
    assert!(names.contains(&"results_lobby_hover"));
    assert!(names.contains(&"match_paused"));
    assert!(names.contains(&"load_screen_failed"));
}

#[test]
fn pre_alpha_acceptance_capture_plan_is_capture_only() {
    let plan = pre_alpha_acceptance_capture_plan(PathBuf::from("ra2"), PathBuf::from("."));
    assert!(plan.actions.iter().all(|a| matches!(a, GuiAction::Capture { .. })));
    assert_eq!(plan.actions.len(), pre_alpha_acceptance_capture_names().len());
}
