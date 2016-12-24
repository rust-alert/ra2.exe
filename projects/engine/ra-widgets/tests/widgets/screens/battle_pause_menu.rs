//! 暂停菜单命中。

use ra_layout::BATTLE_PAUSE_MENU_BUTTON_IDS;
use ra_widgets::{
    battle_pause_menu::{BattlePauseMenuHit, button_rects, hit_at},
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
    let load = rects[0];
    let game_controls = rects[3];
    let abort = rects[4];
    let resume = rects[5];
    assert!(resume.y > abort.y, "resume should sit below abort");
    assert_eq!(game_controls.x, 800 - 147);
    assert_eq!(hit_at(800, 600, load.x + 4, load.y + 4), None, "disabled load should not hit");
    assert_eq!(
        hit_at(800, 600, game_controls.x + 4, game_controls.y + 4),
        Some(BattlePauseMenuHit::GameControls)
    );
    assert_eq!(hit_at(800, 600, resume.x + 4, resume.y + 4), Some(BattlePauseMenuHit::Resume));
}

#[test]
fn compose_battle_pause_menu_dims_and_paints_sidebttn_cells() {
    let page = compose_battle_pause_menu_overlay(800, 600, None, Some("game_controls"), None, None, None).unwrap();
    assert_eq!(page.width(), 800);
    assert_eq!(page.height(), 600);
    assert!(page.as_raw()[3] > 0, "dim overlay alpha");
    let game_controls = button_rects(800, 600)[3];
    let x = (game_controls.x + 4) as u32;
    let y = (game_controls.y + 4) as u32;
    let di = ((y * 800 + x) * 4) as usize;
    assert!(page.as_raw()[di + 3] >= 200, "button alpha={}", page.as_raw()[di + 3]);
}
