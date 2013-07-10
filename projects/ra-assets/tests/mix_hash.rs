//! 集成测试：原 `src/mix_hash.rs` 内联测试迁出。

use ra_assets::*;

#[test]
fn crc32_known_vector() {
    assert_eq!(crc32(b"123456789"), 0xCBF43926);
}

#[test]
fn mix_hash_case_insensitive() {
    assert_eq!(mix_hash("rules.ini"), mix_hash("RULES.INI"));
}

#[test]
fn padding_for_rules_ini() {
    let padded = westwood_pad(b"RULES.INI");
    assert_eq!(padded.len(), 12);
    assert_eq!(padded[9], 0x01);
    assert_eq!(padded[10], b'I');
    assert_eq!(padded[11], b'I');
}
