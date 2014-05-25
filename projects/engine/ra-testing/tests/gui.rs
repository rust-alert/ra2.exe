//! 集成测试：原 `src/gui.rs` 内联测试迁出。

use std::path::PathBuf;

use ra_testing::*;

#[test]
fn pre_alpha_capture_names_cover_entry_flow() {
    let names = pre_alpha_acceptance_capture_names();
    assert!(names.contains(&"main_menu"));
    assert!(names.contains(&"skirmish_lobby"));
    assert!(names.contains(&"match"));
    assert!(names.contains(&"results"));
    assert!(names.len() >= 8);
}

#[test]
fn pre_alpha_capture_plan_lists_one_capture_per_name() {
    let plan = pre_alpha_acceptance_capture_plan(PathBuf::from("ra2"), PathBuf::from("."));
    assert_eq!(plan.name, "pre-alpha-acceptance-captures");
    assert_eq!(plan.actions.len(), pre_alpha_acceptance_capture_names().len());
    for (action, name) in plan.actions.iter().zip(pre_alpha_acceptance_capture_names()) {
        match action {
            GuiAction::Capture { name: captured } => assert_eq!(captured, name),
            other => panic!("expected Capture, got {other:?}"),
        }
    }
}
