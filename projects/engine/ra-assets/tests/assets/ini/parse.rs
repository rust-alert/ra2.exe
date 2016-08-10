//! 自 `engine/ra-assets/src/ini/parse.rs` 迁出的单元测试（集成测试 crate）。

// 自 engine/ra-assets/src/ini/parse.rs :: tests
use ra_assets::{SourceId, ini::parse::*};

#[test]
fn section_trailing_slash_slash_comment() {
    let raw = "[MirageWH]    // Supposed to be a heat ray.\nVerses=100%,100%,80%\n";
    let doc = parse_westwood(raw.as_bytes(), SourceId::default()).unwrap();
    assert!(doc.has_section("MirageWH"));
    assert_eq!(doc.get("MirageWH", "Verses"), Some("100%,100%,80%"));
}

#[test]
fn drops_decorative_lines_without_equal() {
    let raw = "[General]\n***Crazy Ivan stuff***\nStrength=100\n// PCG comment\nName=Foo\n842-GAWETH_ED\n";
    let doc = parse_westwood(raw.as_bytes(), SourceId::default()).unwrap();
    assert_eq!(doc.get("General", "Strength"), Some("100"));
    assert_eq!(doc.get("General", "Name"), Some("Foo"));
    assert_eq!(doc.section("General").unwrap().entries.len(), 2);
}

#[test]
fn empty_value_and_semicolon_comment() {
    let raw = "[A]\nEmpty=\nName=Tank ; unit name\n";
    let doc = parse_westwood(raw.as_bytes(), SourceId::default()).unwrap();
    assert_eq!(doc.get("A", "Empty"), Some(""));
    assert_eq!(doc.get("A", "Name"), Some("Tank"));
}

#[test]
fn keeps_url_like_double_slash_in_value() {
    let raw = "[Net]\nUrl=http://example.com/path\n";
    let doc = parse_westwood(raw.as_bytes(), SourceId::default()).unwrap();
    assert_eq!(doc.get("Net", "Url"), Some("http://example.com/path"));
}

#[test]
fn leading_properties_before_first_section() {
    let raw = "Pre=1\n[S]\nK=2\n";
    let doc = parse_westwood(raw.as_bytes(), SourceId::default()).unwrap();
    assert_eq!(doc.leading.len(), 1);
    assert_eq!(doc.leading[0].key_raw, "Pre");
    assert_eq!(doc.get("S", "K"), Some("2"));
}

#[test]
fn span_uses_decoded_utf8_offsets() {
    let raw = "[A]\nKey=1\n";
    let doc = parse_westwood(raw.as_bytes(), SourceId::default()).unwrap();
    let sec = doc.section("A").unwrap();
    let span = sec.span.unwrap();
    assert_eq!(&raw[span.start..span.end], "[A]");
    let entry = &sec.entries[0];
    let es = entry.span.unwrap();
    assert_eq!(&raw[es.start..es.end], "Key=1");
}

#[test]
fn windows_1252_high_bytes_decode_before_parse() {
    // `0x85` 在 Windows-1252 为省略号；非法 UTF-8。
    let raw = b"[VOX]\nText=battlefield control\x85standby.\nRussian=csof016\n";
    let doc = parse_westwood(raw, SourceId::default()).unwrap();
    assert_eq!(doc.get("VOX", "Text"), Some("battlefield control\u{2026}standby."));
    assert_eq!(doc.get("VOX", "Russian"), Some("csof016"));
}
