//! 集成测试：原 `src/image/bink_residue.rs` 内联测试迁出。

use ra_assets::*;

#[test]
fn residue_zero_masks_consumes_mask_bits_only() {
    // masks_count=0：读 3 位 mask 起点后立刻因 masks_count<0 退出。
    // 实际 masks_count 由调用方传入；此处验证空块可安全调用。
    let data = [0u8]; // 3 位 mask 指数 + 若干 0
    let mut r = BitReader::from_bytes(&data);
    let mut block = [0i16; 64];
    read_residue(&mut r, &mut block, 0).unwrap();
    assert!(block.iter().all(|&b| b == 0));
}
