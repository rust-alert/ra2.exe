//! 集成测试：原 `src/screenshot.rs` 内联测试迁出。

use std::path::{Path, PathBuf};

use ra_widgets::original_screen::OriginalScreen;
use ra_napi::host::screenshot::{AutoScreenshotTracker, is_key_screen, planned_path};

#[test]
fn key_screens_include_main_menu_and_results() {
    assert!(is_key_screen(OriginalScreen::MainMenu));
    assert!(is_key_screen(OriginalScreen::Results));
    assert!(is_key_screen(OriginalScreen::Splash));
}

#[test]
fn planned_path_uses_screen_and_millis() {
    let p = planned_path(Path::new("screenshots"), "main_menu", 42);
    assert_eq!(p, PathBuf::from("screenshots/main_menu_42.png"));
}

#[test]
fn tracker_fires_once_per_screen() {
    let mut t = AutoScreenshotTracker::default();
    assert!(t.should_capture_if(OriginalScreen::MainMenu, true));
    assert!(!t.should_capture_if(OriginalScreen::MainMenu, true));
    assert!(!t.should_capture_if(OriginalScreen::MainMenu, false));
}
