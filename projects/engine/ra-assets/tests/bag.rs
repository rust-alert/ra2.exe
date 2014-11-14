//! 集成测试：原 `src/audio/bag.rs` 内联测试迁出。

use ra_assets::AudioIndex;

fn build_idx_v1(entries: &[(&str, u32, u32, u32, u32)]) -> Vec<u8> {
    let mut idx = Vec::new();
    idx.extend_from_slice(&0u32.to_le_bytes());
    idx.extend_from_slice(&1u32.to_le_bytes());
    idx.extend_from_slice(&(entries.len() as u32).to_le_bytes());
    for &(name, offset, size, rate, flags) in entries {
        let mut name_buf = [0u8; 16];
        let nb = name.as_bytes();
        let n = nb.len().min(15);
        name_buf[..n].copy_from_slice(&nb[..n]);
        idx.extend_from_slice(&name_buf);
        idx.extend_from_slice(&offset.to_le_bytes());
        idx.extend_from_slice(&size.to_le_bytes());
        idx.extend_from_slice(&rate.to_le_bytes());
        idx.extend_from_slice(&flags.to_le_bytes());
    }
    idx
}

#[test]
fn parse_and_decode_pcm16() {
    let idx = build_idx_v1(&[("CLICK", 0, 4, 22050, 0x04)]);
    let bag = {
        let mut b = Vec::new();
        b.extend_from_slice(&1000i16.to_le_bytes());
        b.extend_from_slice(&(-1000i16).to_le_bytes());
        b
    };
    let index = AudioIndex::parse(&idx, bag).unwrap();
    assert_eq!(index.len(), 1);
    let pcm = index.decode("click").unwrap();
    assert_eq!(pcm.sample_rate, 22050);
    assert_eq!(pcm.samples, vec![1000, -1000]);
}

#[test]
fn reject_bad_version() {
    let mut idx = build_idx_v1(&[]);
    idx[4..8].copy_from_slice(&99u32.to_le_bytes());
    assert!(AudioIndex::parse(&idx, vec![]).is_none());
}
