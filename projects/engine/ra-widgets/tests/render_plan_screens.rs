//! `OriginalScreen` → `RenderPlan` 工厂映射。

use ra_widgets::{OriginalScreen, RenderPlan};

#[test]
fn for_original_screen_covers_menu_pages() {
    let screens = [
        OriginalScreen::MainMenu,
        OriginalScreen::SinglePlayerMenu,
        OriginalScreen::Campaign,
        OriginalScreen::SkirmishLobby,
        OriginalScreen::ChooseMap,
        OriginalScreen::Options,
        OriginalScreen::ExitConfirm,
        OriginalScreen::LoadScreen,
        OriginalScreen::Network,
    ];
    for screen in screens {
        let plan = RenderPlan::for_original_screen(screen).expect(screen.as_str());
        assert!(
            !plan.commands.is_empty(),
            "{} should yield placeholder commands",
            screen.as_str()
        );
        let diag = RenderPlan::diagnostic_for_original_screen(screen).expect(screen.as_str());
        assert!(diag.rect_of("background").is_none());
        assert!(diag.rect_of("movie").is_none());
    }
}

#[test]
fn for_original_screen_skips_runtime_surfaces() {
    assert!(RenderPlan::for_original_screen(OriginalScreen::Splash).is_none());
    assert!(RenderPlan::for_original_screen(OriginalScreen::Battle).is_none());
    assert!(RenderPlan::for_original_screen(OriginalScreen::Results).is_none());
}

#[test]
fn choose_map_and_skirmish_factories_match_dialog_roots() {
    assert!(RenderPlan::choose_map_placeholders().rect_of("use_map").is_some());
    assert!(RenderPlan::skirmish_lobby_placeholders().rect_of("start").is_some());
}
