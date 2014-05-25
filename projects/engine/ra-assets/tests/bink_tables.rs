//! 集成测试：原 `src/image/bink_tables.rs` 内联测试迁出。

use ra_assets::*;

#[test]
fn sixteen_trees_of_sixteen_symbols() {
    assert_eq!(BINK_TREE_BITS.len(), 16);
    assert_eq!(BINK_TREE_LENS.len(), 16);
    for t in 0..16 {
        assert_eq!(BINK_TREE_BITS[t].len(), 16);
        assert_eq!(BINK_TREE_LENS[t].len(), 16);
        let max = *BINK_TREE_LENS[t].iter().max().unwrap();
        assert!(max > 0 && max <= 13);
    }
    assert_eq!(BINK_RLELENS, [4, 8, 12, 32]);
}
