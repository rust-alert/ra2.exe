//! 集成测试：原 `src/image/bink_quant.rs` 内联测试迁出。

use ra_assets::*;

#[test]
fn quant_tables_shape() {
    assert_eq!(BINK_INTRA_QUANT.len(), 16);
    assert_eq!(BINK_INTER_QUANT.len(), 16);
    assert_eq!(BINK_INTRA_QUANT[0][0], 0x010000);
    assert_ne!(BINK_INTRA_QUANT[0][1], BINK_INTER_QUANT[0][1]);
}
