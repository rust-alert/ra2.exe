//! 集成测试：原 `src/shp/decode.rs` 内联测试迁出。

use ra_assets::*;

#[test]
fn basic_rle_row() {
    let data = [6, 0, 0, 1, 5, 3];
    assert_eq!(decode_rle_frame(&data, 3, 1).unwrap(), vec![0, 5, 3]);
}

#[test]
fn all_transparent_row() {
    let data = [4, 0, 0, 4];
    assert_eq!(decode_rle_frame(&data, 4, 1).unwrap(), vec![0, 0, 0, 0]);
}
