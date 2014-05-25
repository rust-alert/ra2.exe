//! 集成测试：原 `src/image/bink_idct.rs` 内联测试迁出。

use ra_assets::*;

#[test]
fn scan_visits_all_64() {
    let mut seen = [false; 64];
    for &i in &BINK_SCAN {
        seen[i as usize] = true;
    }
    assert!(seen.iter().all(|&x| x));
}

#[test]
fn dc_only_idct_is_flat() {
    let mut block = [0i32; 64];
    block[0] = 1024;
    bink_idct(&mut block);
    let first = block[0];
    for &v in &block {
        assert_eq!(v, first);
    }
    // 仅 DC=1024：列广播后行变换得 (1024+127)>>8 == 4
    assert_eq!(first, 4);
}

#[test]
fn idct_put_dc_fills_bytes() {
    let mut block = [0i32; 64];
    block[0] = 1024;
    let mut dst = [0u8; 64];
    idct_put(&mut dst, 8, &mut block);
    assert!(dst.iter().all(|&b| b == 4));
}
