//! 集成测试：原 `src/pal.rs` 内联测试迁出。

use ra_assets::*;

const PAL_FILE_SIZE: usize = 256 * 3;

#[test]
fn rejects_wrong_size() {
    assert!(Palette::parse(&[0u8; 10]).is_err());
}

#[test]
fn expands_vga_components_full_range() {
    let mut data = [0u8; PAL_FILE_SIZE];
    data[3] = 63;
    data[4] = 31;
    data[5] = 1;
    let pal = Palette::parse(&data).unwrap();
    assert_eq!(pal.colors[0].a, 0);
    // `(v & 63) * 255 / 63`：63→255，31→125，1→4。
    assert_eq!(pal.colors[1], Rgba::rgb(255, 125, 4));
}

#[test]
fn magenta_key_is_transparent() {
    let mut data = [0u8; PAL_FILE_SIZE];
    data[3..6].copy_from_slice(&[63, 0, 63]);
    let pal = Palette::parse(&data).unwrap();
    assert_eq!(pal.colors[1].a, 0);
    assert_eq!(pal.colors[1].r, 255);
    assert_eq!(pal.colors[1].b, 255);
}
