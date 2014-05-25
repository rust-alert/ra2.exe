//! 集成测试：原 `src/image/bink_patterns.rs` 内联测试迁出。

use ra_assets::*;

#[test]
fn sixteen_patterns_cover_0_63() {
    assert_eq!(BINK_RUN_PATTERNS.len(), 16);
    for pat in &BINK_RUN_PATTERNS {
        let mut seen = [false; 64];
        for &i in pat {
            seen[i as usize] = true;
        }
        assert!(seen.iter().all(|&x| x));
    }
}
