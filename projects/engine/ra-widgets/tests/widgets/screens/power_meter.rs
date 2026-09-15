//! 侧栏电表满度 / 变色。

use ra_widgets::battle_hud::{POWER_METER_FULL_LEVEL, PowerMeterColor, power_meter_paint};

#[test]
fn surplus_is_green_partial_fill() {
    let band = 200;
    let paint = power_meter_paint(band, 1000, 250, false);
    assert_eq!(paint.color, PowerMeterColor::Green);
    // 标称 = 1000 → 半高；耗电占 1/4 → 填色为半高的 1/4。
    let meter_h = band * 1000 / POWER_METER_FULL_LEVEL;
    assert_eq!(paint.fill_h, meter_h * 250 / 1000);
}

#[test]
fn near_capacity_is_yellow() {
    let paint = power_meter_paint(100, 100, 80, false);
    assert_eq!(paint.color, PowerMeterColor::Yellow);
    assert_eq!(paint.fill_h, 100 * 100 / POWER_METER_FULL_LEVEL * 80 / 100);
}

#[test]
fn low_power_is_red_full_of_drain_scale() {
    let band = 200;
    let paint = power_meter_paint(band, 100, 400, true);
    assert_eq!(paint.color, PowerMeterColor::Red);
    // 标称 = 400，填满该标称段。
    let meter_h = band * 400 / POWER_METER_FULL_LEVEL;
    assert_eq!(paint.fill_h, meter_h);
}

#[test]
fn idle_zero_is_gray_empty() {
    let paint = power_meter_paint(180, 0, 0, false);
    assert_eq!(paint.color, PowerMeterColor::Gray);
    assert_eq!(paint.fill_h, 0);
}

#[test]
fn blackout_low_power_with_drain_is_red() {
    // 断电：有效供电 0，仍按耗电标称画红条。
    let paint = power_meter_paint(200, 0, 500, true);
    assert_eq!(paint.color, PowerMeterColor::Red);
    assert!(paint.fill_h > 0);
}
