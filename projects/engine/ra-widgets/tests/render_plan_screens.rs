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

#[test]
fn diagnostic_recolors_rail_buttons() {
    let plan = RenderPlan::diagnostic_for_original_screen(OriginalScreen::MainMenu)
        .expect("main menu");
    let exit = plan
        .commands
        .iter()
        .find_map(|cmd| match cmd {
            ra_widgets::RenderCommand::SolidRect { id, color, .. } if id.0 == "exit" => {
                Some(*color)
            }
            _ => None,
        })
        .expect("exit command");
    assert_eq!(exit, [196, 148, 48, 255]);
}

#[test]
fn recolor_ids_only_touches_named_commands() {
    let base = RenderPlan::choose_map_placeholders();
    let tinted = base.recolor_ids(&["use_map"], [1, 2, 3, 4]);
    let use_map = tinted
        .commands
        .iter()
        .find_map(|cmd| match cmd {
            ra_widgets::RenderCommand::SolidRect { id, color, .. } if id.0 == "use_map" => {
                Some(*color)
            }
            _ => None,
        })
        .expect("use_map");
    let cancel = tinted
        .commands
        .iter()
        .find_map(|cmd| match cmd {
            ra_widgets::RenderCommand::SolidRect { id, color, .. } if id.0 == "cancel" => {
                Some(*color)
            }
            _ => None,
        })
        .expect("cancel");
    assert_eq!(use_map, [1, 2, 3, 4]);
    assert_eq!(cancel, [80, 80, 80, 255]);
}
