//! 等距格子 ↔ 屏幕像素。

/// 钻石宽（像素）。
pub const TILE_WIDTH: i32 = 60;
/// 钻石高（像素）。
pub const TILE_HEIGHT: i32 = 30;
/// 每级高度对应的垂直偏移。
pub const HEIGHT_STEP: i32 = 15;

/// 格子坐标转屏幕左上角（钻石包围盒原点）。
pub fn iso_to_screen(rx: i32, ry: i32, z: u8) -> (i32, i32) {
    let sx = (rx - ry) * (TILE_WIDTH / 2) - TILE_WIDTH / 2;
    let sy = (rx + ry) * (TILE_HEIGHT / 2) + TILE_HEIGHT / 2 - i32::from(z) * HEIGHT_STEP;
    (sx, sy)
}

/// 屏幕像素（近似钻石中心）逆变换为格子；`z` 与 `iso_to_screen` 一致。
pub fn screen_to_iso(px: i32, py: i32, z: u8) -> (i32, i32) {
    // 钻石中心：cx = (rx-ry)*30，cy = (rx+ry)*15 + 30 - z*15
    let cx = f64::from(px);
    let cy = f64::from(py);
    let diff = cx / f64::from(TILE_WIDTH / 2);
    let sum = (cy - 30.0 + f64::from(z) * f64::from(HEIGHT_STEP)) / f64::from(TILE_HEIGHT / 2);
    let rx = ((sum + diff) / 2.0).round() as i32;
    let ry = ((sum - diff) / 2.0).round() as i32;
    (rx, ry)
}
