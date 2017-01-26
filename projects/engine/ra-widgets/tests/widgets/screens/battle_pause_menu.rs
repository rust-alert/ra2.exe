//! 暂停菜单命中。

use ra_layout::BATTLE_PAUSE_MENU_BUTTON_IDS;
use ra_widgets::{
    battle_pause_menu::{BattlePauseMenuHit, button_rects, entry_enabled, hit_at},
    compose::compose_battle_pause_menu_overlay,
};

#[test]
fn battle_pause_menu_has_six_buttons() {
    assert_eq!(BATTLE_PAUSE_MENU_BUTTON_IDS.len(), 6);
    let rects = button_rects(800, 600);
    assert_eq!(rects.len(), 6);
}

#[test]
fn battle_pause_menu_hits_resume_and_game_controls_from_snapshot() {
    let rects = button_rects(800, 600);
    let game_controls = rects[0];
    let load = rects[1];
    let abort = rects[4];
    let resume = rects[5];
    assert!(resume.y > abort.y, "resume should sit below abort");
    assert_eq!(game_controls.x, 800 - 147);
    assert_eq!(hit_at(800, 600, load.x + 4, load.y + 4, /* saves_allowed */ false), None, "skirmish load should not hit");
    assert_eq!(hit_at(800, 600, load.x + 4, load.y + 4, /* saves_allowed */ true), Some(BattlePauseMenuHit::Load), "campaign load should hit");
    assert!(!entry_enabled("load", false));
    assert!(entry_enabled("load", true));
    assert_eq!(hit_at(800, 600, game_controls.x + 4, game_controls.y + 4, false), Some(BattlePauseMenuHit::GameControls));
    assert_eq!(hit_at(800, 600, resume.x + 4, resume.y + 4, false), Some(BattlePauseMenuHit::Resume));
}

#[test]
fn compose_battle_pause_menu_dims_and_paints_sidebttn_cells() {
    let page =
        compose_battle_pause_menu_overlay(800, 600, None, Some("game_controls"), None, None, None, None, None, /* saves_allowed */ false)
            .unwrap();
    assert_eq!(page.width(), 800);
    assert_eq!(page.height(), 600);
    assert!(page.as_raw()[3] > 0, "dim overlay alpha");
    let game_controls = button_rects(800, 600)[0];
    let x = (game_controls.x + 4) as u32;
    let y = (game_controls.y + 4) as u32;
    let di = ((y * 800 + x) * 4) as usize;
    assert!(page.as_raw()[di + 3] >= 200, "button alpha={}", page.as_raw()[di + 3]);
    // 无 hud chrome 时右轨保持透明（有 chrome 时由 pause hub 填实）。
    let rail_x = 790u32;
    let rail_y = 100u32;
    let ri = ((rail_y * 800 + rail_x) * 4) as usize;
    assert_eq!(page.as_raw()[ri + 3], 0, "rail should stay transparent without chrome");
}
