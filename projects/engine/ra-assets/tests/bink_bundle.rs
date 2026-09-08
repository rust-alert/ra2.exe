//! 集成测试：原 `src/image/bink_bundle.rs` 内联测试迁出。

use ra_assets::*;

#[test]
fn init_lengths_for_320_wide() {
    let (mut bundles, _) = alloc_bundles(320, 240);
    let bw = (320 + 7) >> 3;
    init_bundle_lengths(&mut bundles, 320, bw);
    assert!(bundles[BinkSrc::BlockTypes as usize].len_bits >= 1);
    assert_eq!(bundles[BinkSrc::BlockTypes as usize].len_bits, bundles[BinkSrc::XOff as usize].len_bits);
}

#[test]
fn read_bundle_identity_tree() {
    let (mut bundles, _) = alloc_bundles(16, 16);
    init_bundle_lengths(&mut bundles, 16, 2);
    // vlc_num = 0
    let data = [0x00u8];
    let mut r = BitReader::from_bytes(&data);
    let mut col_high = std::array::from_fn(|_| HuffmanTree::default());
    let mut col_last = 0u8;
    read_bundle(&mut r, &mut bundles, &mut col_high, &mut col_last, BinkSrc::BlockTypes as usize).unwrap();
    assert_eq!(bundles[0].tree.vlc_num, 0);
    assert!(!bundles[0].skip_fills);
}
