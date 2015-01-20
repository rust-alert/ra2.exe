//! 集成测试：原 `src/image/csf.rs` 内联测试迁出。

use ra_assets::image::csf::{CsfFile, LABEL_MAGIC, STRING_MAGIC};

const HEADER_MAGIC: u32 = 0x4353_4620; // " FSC"

fn encode_not_utf16(s: &str) -> Vec<u8> {
    let mut out = Vec::new();
    for u in s.encode_utf16() {
        out.extend_from_slice(&(!u).to_le_bytes());
    }
    out
}

fn tiny_csf() -> Vec<u8> {
    let label = b"GUI:SINGLEPLAYER";
    let value = encode_not_utf16("Single Player");
    let mut data = Vec::new();
    data.extend_from_slice(&HEADER_MAGIC.to_le_bytes());
    data.extend_from_slice(&2u32.to_le_bytes()); // version
    data.extend_from_slice(&1u32.to_le_bytes()); // labels
    data.extend_from_slice(&1u32.to_le_bytes()); // strings
    data.extend_from_slice(&0u32.to_le_bytes()); // reserved
    data.extend_from_slice(&0u32.to_le_bytes()); // language
    data.extend_from_slice(&LABEL_MAGIC.to_le_bytes());
    data.extend_from_slice(&1u32.to_le_bytes()); // pairs
    data.extend_from_slice(&(label.len() as u32).to_le_bytes());
    data.extend_from_slice(label);
    data.extend_from_slice(&STRING_MAGIC.to_le_bytes());
    data.extend_from_slice(&((value.len() / 2) as u32).to_le_bytes());
    data.extend_from_slice(&value);
    data
}

#[test]
fn parse_and_lookup_is_case_insensitive() {
    let csf = CsfFile::parse(&tiny_csf()).unwrap();
    assert_eq!(csf.len(), 1);
    assert_eq!(csf.get("gui:singleplayer"), Some("Single Player"));
    assert_eq!(csf.get("GUI:SinglePlayer"), Some("Single Player"));
}

#[test]
fn text_table_escapes_newlines_and_sorts_keys() {
    let csf = CsfFile::parse(&tiny_csf()).unwrap();
    let text = csf.to_text_table();
    assert_eq!(text, "GUI:SINGLEPLAYER=Single Player\n");
}

#[test]
fn parse_rejects_bad_magic() {
    let err = CsfFile::parse(&[0u8; 24]).unwrap_err();
    assert!(err.to_string().contains("魔数"));
}
