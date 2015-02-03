//! 暂停菜单布局与命中。

use ra_widgets::battle_pause_menu::{BattlePauseMenuHit, hit_at, layout};
use ra_widgets::ui_compose::compose_battle_pause_menu_overlay;
use ra_layout::BATTLE_PAUSE_MENU_BUTTON_IDS;

#[test]
fn battle_pause_menu_layout_six_buttons_and_resume_at_bottom() {
    let l = layout();
    assert_eq!(BATTLE_PAUSE_MENU_BUTTON_IDS.len(), 6);
    assert!(l.dim.w > 0);
    assert!(l.sidebar.w > 0);
    let resume = l.buttons[5];
    let abort = l.buttons[4];
    assert!(resume.y > abort.y, "resume should sit below abort");
    assert_eq!(l.hit_entry_id(resume.x + 4, resume.y + 4), Some("resume"));
    assert_eq!(hit_at(l, resume.x + 4, resume.y + 4), Some(BattlePauseMenuHit::Resume));
    assert_eq!(hit_at(l, l.buttons[0].x + 4, l.buttons[0].y + 4), Some(BattlePauseMenuHit::Options));
}

#[test]
fn compose_battle_pause_menu_dims_left_and_paints_rail() {
    let page = compose_battle_pause_menu_overlay(800, 600, None, Some("options"), None, None, None).unwrap();
    assert_eq!(page.width(), 800);
    assert_eq!(page.height(), 600);
    // 左区压暗罩有 alpha。
    assert!(page.as_raw()[3] > 0, "dim overlay alpha");
    // 右栏内不透明。
    let x = 760u32;
    let y = 220u32;
    let di = ((y * 800 + x) * 4) as usize;
    assert!(page.as_raw()[di + 3] >= 200, "sidebar alpha={}", page.as_raw()[di + 3]);
}
