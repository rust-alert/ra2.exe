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

#[test]
fn promote_ids_to_sprites_keeps_rect_and_skips_raster() {
    use ra_widgets::RenderCommand;

    let full = RenderPlan::choose_map_placeholders();
    let plan = full
        .retaining_ids(&["use_map"])
        .promote_ids_to_sprites(&["use_map"]);
    assert!(matches!(
        plan.commands.as_slice(),
        [RenderCommand::SpriteRect { slot, .. }] if slot == "use_map"
    ));
    assert_eq!(plan.rect_of("use_map"), full.rect_of("use_map"));
    let page = plan.rasterize_solids(800, 600).expect("page");
    assert!(page.as_raw().iter().all(|&b| b == 0));
}

#[test]
fn button_sprite_plan_and_paint_sprites_into() {
    use ra_renderer::RgbaImage;
    use ra_widgets::RenderCommand;

    let plan = RenderPlan::choose_map_placeholders().button_sprite_plan(&["use_map"]);
    assert!(matches!(
        plan.commands.as_slice(),
        [RenderCommand::SpriteRect { slot, .. }] if slot == "use_map"
    ));
    let cell = plan.rect_px_of("use_map").expect("rect_px");
    assert!(cell.w > 0 && cell.h > 0);

    let sprite = RgbaImage::from_raw(2, 2, vec![255, 0, 0, 255, 255, 0, 0, 255, 255, 0, 0, 255, 255, 0, 0, 255])
        .expect("sprite");
    let mut page = RgbaImage::from_raw(800, 600, vec![0u8; 800 * 600 * 4]).expect("page");
    plan.paint_sprites_into(&mut page, |slot| {
        if slot == "use_map" {
            Some(&sprite)
        } else {
            None
        }
    });
    let i = ((cell.y as u32 * 800 + cell.x as u32) * 4) as usize;
    assert_eq!(&page.as_raw()[i..i + 4], &[255, 0, 0, 255]);
}
