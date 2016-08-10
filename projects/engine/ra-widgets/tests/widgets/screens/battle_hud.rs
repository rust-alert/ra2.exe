//! 自顶层 `battle_hud_unit.rs`。

use ra_layout::{BattleHudChromeMetrics, RectPx, cameo_slot_rect, rect_px_from_snapshot, solve_battle_hud_with_metrics};
use ra_renderer::RgbaImage;

use ra_widgets::battle_hud::{BattleHudHit, cameo_ready_flash_on, hit_at_with_chrome, paint_cameo_progress_clock, radar_open_frame_range};

#[test]
fn ready_flash_toggles_with_tick() {
    assert!(cameo_ready_flash_on(0));
    assert!(cameo_ready_flash_on(7));
    assert!(!cameo_ready_flash_on(8));
    assert!(!cameo_ready_flash_on(15));
    assert!(cameo_ready_flash_on(16));
}

#[test]
fn progress_clock_covers_when_empty() {
    let mut page = RgbaImage::from_raw(16, 16, vec![200u8; 16 * 16 * 4]).unwrap();
    paint_cameo_progress_clock(&mut page, RectPx::new(0, 0, 16, 16), 0.0);
    let px = page.as_raw();
    // 中心附近应被压暗。
    let i = ((8u32 * 16 + 8) * 4) as usize;
    assert!(px[i] < 200, "clock wipe should darken uncovered cells");
}

#[test]
fn progress_clock_skips_when_complete() {
    let mut page = RgbaImage::from_raw(16, 16, vec![200u8; 16 * 16 * 4]).unwrap();
    paint_cameo_progress_clock(&mut page, RectPx::new(0, 0, 16, 16), 1.0);
    let px = page.as_raw();
    let i = ((8u32 * 16 + 8) * 4) as usize;
    assert_eq!(px[i], 200);
}

#[test]
fn radar_open_frame_range_skips_emblem_and_blank_tail() {
    assert_eq!(radar_open_frame_range(0), 1..1);
    assert_eq!(radar_open_frame_range(1), 1..1);
    assert_eq!(radar_open_frame_range(2), 1..2);
    assert_eq!(radar_open_frame_range(8), 1..7);
}

#[test]
fn hit_tabs_and_cameo_slots() {
    let metrics = BattleHudChromeMetrics::sidec01();
    let snap = solve_battle_hud_with_metrics(800, 600, metrics);
    let tab0 = rect_px_from_snapshot(&snap, "tab00");
    assert_eq!(hit_at_with_chrome(&snap, None, metrics.power_w, 4, tab0.x + 1, tab0.y + 1), Some(BattleHudHit::SidebarTab(0)));
    let band = rect_px_from_snapshot(&snap, "cameo_band");
    let cell = cameo_slot_rect(band, metrics.power_w, 0).expect("slot0");
    assert_eq!(hit_at_with_chrome(&snap, None, metrics.power_w, 2, cell.x + 1, cell.y + 1), Some(BattleHudHit::Cameo(0)));
    assert_eq!(hit_at_with_chrome(&snap, None, metrics.power_w, 0, cell.x + 1, cell.y + 1), None, "空列表时 cameo 槽应吞掉点击");
}
