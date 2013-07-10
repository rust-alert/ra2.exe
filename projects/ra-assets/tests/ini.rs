//! 集成测试：原 `src/ini.rs` 内联测试迁出。

use ra_assets::*;

#[test]
fn numbered_concat_sorts_numerically() {
    let doc = IniDocument::parse(b"[IsoMapPack5]\n10=C\n2=B\n1=A\n").unwrap();
    assert_eq!(doc.numbered_section_concat("IsoMapPack5").as_deref(), Some("ABC"));
}
