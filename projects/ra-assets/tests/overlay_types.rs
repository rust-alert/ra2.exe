//! 集成测试：原 `src/overlay_types.rs` 内联测试迁出。

use ra_assets::*;

#[test]
fn parse_sparse_ids() {
    let doc = IniDocument::parse(b"[OverlayTypes]\n3=GAWALL\n105=TIB01\n1=GASAND\n").unwrap();
    let reg = OverlayTypeRegistry::from_rules(&doc);
    assert_eq!(reg.len(), 3);
    assert_eq!(reg.name(1), Some("GASAND"));
    assert_eq!(reg.name(3), Some("GAWALL"));
    assert_eq!(reg.name(105), Some("TIB01"));
    assert_eq!(reg.name(2), None);
}
