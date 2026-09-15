//! 选中供电建筑的电力 / 负载文案。

use ra_widgets::{format_csf_percent_d, selection_power_drain_caption, structure_selection_center_preview};

#[test]
fn format_replaces_percent_d_in_order() {
    assert_eq!(format_csf_percent_d("电力 = %d 负载 = %d", &[200, 0]), "电力 = 200 负载 = 0");
    assert_eq!(format_csf_percent_d("電力=%d \n 負載=%d", &[150, 40]), "電力=150 \n 負載=40");
}

#[test]
fn caption_falls_back_without_csf() {
    let text = selection_power_drain_caption(None, 200, 0);
    assert_eq!(text, "电力 = 200 负载 = 0");
}

#[test]
fn caption_expands_literal_backslash_n() {
    assert_eq!(format_csf_percent_d("电力=%d \\n 负载=%d", &[1, 2]).replace("\\n", "\n"), "电力=1 \n 负载=2");
}

#[test]
fn structure_center_sits_above_footprint() {
    let (cx, cy) = structure_selection_center_preview(100, 80, 2, 2, 3);
    assert!((cx - 100.0).abs() < 0.01);
    assert!(cy < 80.0 + (2.0 + 2.0) * 7.5);
}
