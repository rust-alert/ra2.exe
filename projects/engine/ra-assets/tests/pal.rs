//! 集成测试：VGA 扩色两种模式。

use ra_assets::*;
use ra_types::VgaExpandMode;

const PAL_FILE_SIZE: usize = 256 * 3;

#[test]
fn rejects_wrong_size() {
    assert!(Palette::parse(&[0u8; 10]).is_err());
}

#[test]
fn full_mode_expands_to_255() {
    let mut data = [0u8; PAL_FILE_SIZE];
    data[3] = 63;
    data[4] = 31;
    data[5] = 1;
    let pal = Palette::parse_with(&data, VgaExpandMode::Full).unwrap();
    assert_eq!(pal.colors[0].a, 0);
    assert_eq!(pal.colors[1], Rgba::rgb(255, 125, 4));
}

#[test]
fn shift2_mode_caps_at_252() {
    let mut data = [0u8; PAL_FILE_SIZE];
    data[3] = 63;
    data[4] = 31;
    data[5] = 1;
    let pal = Palette::parse_with(&data, VgaExpandMode::Shift2).unwrap();
    assert_eq!(pal.colors[1], Rgba::rgb(252, 124, 4));
}

#[test]
fn default_parse_follows_set_default_vga_expand() {
    set_default_vga_expand(VgaExpandMode::Shift2);
    assert_eq!(default_vga_expand(), VgaExpandMode::Shift2);
    let mut data = [0u8; PAL_FILE_SIZE];
    data[3] = 63;
    let pal = Palette::parse(&data).unwrap();
    assert_eq!(pal.colors[1].r, 252);
    set_default_vga_expand(VgaExpandMode::Full);
    let pal = Palette::parse(&data).unwrap();
    assert_eq!(pal.colors[1].r, 255);
}

#[test]
fn magenta_key_is_transparent() {
    let mut data = [0u8; PAL_FILE_SIZE];
    data[3..6].copy_from_slice(&[63, 0, 63]);
    let pal = Palette::parse_with(&data, VgaExpandMode::Full).unwrap();
    assert_eq!(pal.colors[1].a, 0);
    assert_eq!(pal.colors[1].r, 255);
    assert_eq!(pal.colors[1].b, 255);
}
