//! 集成测试：原 `src/ui_typewriter.rs` 内联测试迁出。

use ra_desktop::ui_typewriter::*;

#[test]
fn zero_duration_is_instant() {
    let mut t = TypewriterText::new(0.0);
    t.set_text("HELLO");
    assert_eq!(t.visible(), "HELLO");
    assert!(!t.tick(1.0));
    assert!(!t.is_animating());
}

#[test]
fn finishes_in_about_point_four_seconds() {
    let mut t = TypewriterText::new(0.4);
    t.set_text("ABCD");
    assert_eq!(t.visible(), "");
    assert!(t.tick(0.05));
    assert!(t.visible().chars().count() >= 1);
    assert!(t.visible().chars().count() < 4);
    assert!(t.tick(0.35));
    assert_eq!(t.visible(), "ABCD");
    assert!(!t.is_animating());
    assert!(!t.tick(0.1));
}

#[test]
fn same_text_keeps_progress() {
    let mut t = TypewriterText::new(0.4);
    t.set_text("XY");
    let _ = t.tick(0.4);
    t.set_text("XY");
    assert_eq!(t.visible(), "XY");
}

#[test]
fn unicode_scalar_steps() {
    let mut t = TypewriterText::new(0.4);
    t.set_text("任务AB");
    let _ = t.tick(0.1);
    assert_eq!(t.visible().chars().count(), 1);
    let _ = t.tick(0.3);
    assert_eq!(t.visible(), "任务AB");
}
