//! 集成测试：原 `src/image/bink_huff.rs` 内联测试迁出。

use ra_assets::*;

#[test]
fn vlc_num_zero_is_identity() {
    // 仅 4 位：vlc_num = 0
    let data = [0x00u8];
    let mut r = BitReader::from_bytes(&data);
    let t = HuffmanTree::read(&mut r).unwrap();
    assert_eq!(t.vlc_num, 0);
    for i in 0..16 {
        assert_eq!(t.syms[i], i as u8);
    }
}
