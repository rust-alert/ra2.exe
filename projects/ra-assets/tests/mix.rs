//! 集成测试：原 `src/mix.rs` 内联测试迁出。

use ra_assets::*;

#[test]
fn parse_old_header_single_entry() {
    // file_count=1, body_size=4, one entry id=1 offset=0 size=4, body=AAAA
    let mut data = Vec::new();
    data.extend_from_slice(&1u16.to_le_bytes());
    data.extend_from_slice(&4u32.to_le_bytes());
    data.extend_from_slice(&1i32.to_le_bytes());
    data.extend_from_slice(&0u32.to_le_bytes());
    data.extend_from_slice(&4u32.to_le_bytes());
    data.extend_from_slice(b"AAAA");
    let mix = MixArchive::parse(data).unwrap();
    assert_eq!(mix.entry_count(), 1);
    assert_eq!(mix.get_by_id(1), Some(b"AAAA".as_slice()));
}
