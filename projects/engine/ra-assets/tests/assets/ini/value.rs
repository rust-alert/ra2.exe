//! `IniValue` 路径元数据。

use ra_assets::*;

#[test]
fn section_value_carries_path_and_span() {
    let doc = IniDocument::parse(b"[General]\nRepairStep=8\n").unwrap();
    let sec = doc.section("General").unwrap();
    let v = sec.value("RepairStep").expect("key");
    assert_eq!(v.raw.trim(), "8");
    assert_eq!(v.section, "General");
    assert_eq!(v.key, "RepairStep");
    assert!(v.span.is_some());
    let t = v.trimmed();
    assert_eq!(t.raw, "8");
    assert_eq!(t.section, "General");
}
