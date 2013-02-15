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

#[cfg(test)]
mod tests {
    use super::*;

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
}
