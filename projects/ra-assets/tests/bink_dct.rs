//! 集成测试：原 `src/image/bink_dct.rs` 内联测试迁出。

use ra_assets::*;

#[test]
fn emptyish_dct_reads_quant_index() {
    // bits=0（4bit 全0 → bits=-1 跳过循环）+ quant=0
    let data = [0u8];
    let mut r = BitReader::from_bytes(&data);
    let mut block = [0i32; 64];
    block[0] = 100;
    let mut coef_idx = [0usize; 64];
    let mut coef_count = 0usize;
    let q = read_dct_coeffs(&mut r, &mut block, &mut coef_idx, &mut coef_count, None).unwrap();
    assert_eq!(q, 0);
    assert_eq!(coef_count, 0);
    assert_eq!(block[0], 100);
}
