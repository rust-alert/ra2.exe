//! 暂停菜单命中。

use ra_layout::{BattleHudChromeMetrics, BATTLE_PAUSE_MENU_BUTTON_IDS};
use ra_widgets::battle_pause_menu::{button_rects, hit_at, BattlePauseMenuHit};
use ra_widgets::compose::compose_battle_pause_menu_overlay;

#[test]
fn battle_pause_menu_hits_resume_and_options_from_snapshot() {
    let metrics = BattleHudChromeMetrics::sidec01();
    assert_eq!(BATTLE_PAUSE_MENU_BUTTON_IDS.len(), 4);
    let rects = button_rects(800, 600, metrics);
    let options = rects[0];
    let abort = rects[2];
    let resume = rects[3];
    assert!(resume.y > abort.y, "resume should sit below abort");
    assert_eq!(
        hit_at(800, 600, metrics, resume.x + 4, resume.y + 4),
        Some(BattlePauseMenuHit::Resume)
    );
    assert_eq!(
        hit_at(800, 600, metrics, options.x + 4, options.y + 4),
        Some(BattlePauseMenuHit::Options)
    );
}

#[test]
fn compose_battle_pause_menu_dims_left_and_paints_rail() {
    let metrics = BattleHudChromeMetrics::sidec01();
    let page =
        compose_battle_pause_menu_overlay(800, 600, None, Some("options"), None, None, None, metrics)
            .unwrap();
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
