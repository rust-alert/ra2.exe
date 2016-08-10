//! 自顶层 `iso_math.rs`。

use ra_map::{TILE_HEIGHT, TILE_WIDTH, iso_to_screen, screen_to_iso};

#[test]
fn origin_cell() {
    let (sx, sy) = iso_to_screen(0, 0, 0);
    assert_eq!(sx, -30);
    assert_eq!(sy, 15);
}

#[test]
fn step_right() {
    let (sx, sy) = iso_to_screen(1, 0, 0);
    assert_eq!(sx, 0);
    assert_eq!(sy, 30);
}

#[test]
fn screen_to_iso_roundtrip_centers() {
    for &(rx, ry) in &[(0, 0), (1, 0), (3, 5), (10, 2)] {
        let (sx, sy) = iso_to_screen(rx, ry, 0);
        let cx = sx + TILE_WIDTH / 2;
        let cy = sy + TILE_HEIGHT / 2;
        assert_eq!(screen_to_iso(cx, cy, 0), (rx, ry));
    }
}
