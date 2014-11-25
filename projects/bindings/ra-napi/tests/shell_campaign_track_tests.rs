//! 集成测试：原 `src/shell.rs` 内联测试迁出。

use ra_layout::ui_layout::campaign_layout;
use ra_napi::host::shell::campaign_difficulty_from_track_x;

#[test]
fn difficulty_track_maps_left_mid_right() {
    let track = campaign_layout(800, 600).difficulty_track;
    assert_eq!(campaign_difficulty_from_track_x(track, track.x + 2), 0);
    assert_eq!(campaign_difficulty_from_track_x(track, track.x + track.w / 2), 1);
    assert_eq!(campaign_difficulty_from_track_x(track, track.x + track.w - 2), 2);
}
