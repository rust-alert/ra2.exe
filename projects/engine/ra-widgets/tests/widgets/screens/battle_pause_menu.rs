//! 暂停菜单命中。

use ra_layout::BATTLE_PAUSE_MENU_BUTTON_IDS;
use ra_widgets::{
    battle_pause_menu::{BattlePauseMenuHit, button_rects, card_rect, hit_at},
    compose::compose_battle_pause_menu_overlay,
};

#[test]
fn battle_pause_menu_hits_resume_and_options_from_snapshot() {
    assert_eq!(BATTLE_PAUSE_MENU_BUTTON_IDS.len(), 4);
    let rects = button_rects(800, 600);
    let options = rects[0];
    let abort = rects[2];
    let resume = rects[3];
    assert!(resume.y > abort.y, "resume should sit below abort");
    assert!(options.x > 200, "options must be centered, not sidebar-pinned");
    assert_eq!(hit_at(800, 600, resume.x + 4, resume.y + 4), Some(BattlePauseMenuHit::Resume));
    assert_eq!(hit_at(800, 600, options.x + 4, options.y + 4), Some(BattlePauseMenuHit::Options));
}

#[test]
fn compose_battle_pause_menu_dims_and_paints_centered_card() {
    let page = compose_battle_pause_menu_overlay(800, 600, None, Some("options"), None, None, None).unwrap();
    assert_eq!(page.width(), 800);
    assert_eq!(page.height(), 600);
    assert!(page.as_raw()[3] > 0, "dim overlay alpha");
    let card = card_rect(800, 600);
    let cx = (card.x + card.w / 2) as u32;
    let cy = (card.y + 8) as u32;
    let di = ((cy * 800 + cx) * 4) as usize;
    assert!(page.as_raw()[di + 3] >= 200, "card alpha={}", page.as_raw()[di + 3]);
    let options = button_rects(800, 600)[0];
    assert!(options.x > 200);
}
