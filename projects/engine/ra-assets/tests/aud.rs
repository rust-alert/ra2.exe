//! 集成测试：原 `src/audio/aud.rs` 内联测试迁出。

use ra_assets::audio::aud::{decode_ima_aud, try_decode_aud};

#[test]
fn rejects_short_and_non_aud() {
    assert!(try_decode_aud(&[0u8; 8]).is_none());
    assert!(try_decode_aud(b"RIFF........").is_none());
}

#[test]
fn carries_ima_state_across_deaf_chunks() {
    // 两块各 1 字节载荷；连续解码应与单块 2 字节一致。
    fn chunk(payload: &[u8]) -> Vec<u8> {
        let mut c = Vec::new();
        c.extend_from_slice(&(payload.len() as u16).to_le_bytes());
        c.extend_from_slice(&((payload.len() * 4) as u16).to_le_bytes());
        c.extend_from_slice(&0x0000_DEAFu32.to_le_bytes());
        c.extend_from_slice(payload);
        c
    }
    let mut hdr = vec![0u8; 12];
    hdr[0..2].copy_from_slice(&22_050u16.to_le_bytes());
    hdr[10] = 0x02;
    hdr[11] = 99u8;
    let one = {
        let mut d = hdr.clone();
        d.extend(chunk(&[0x12, 0x34]));
        d
    };
    let two = {
        let mut d = hdr;
        d.extend(chunk(&[0x12]));
        d.extend(chunk(&[0x34]));
        d
    };
    let a = decode_ima_aud(&one, 22_050, 0x02).expect("one");
    let b = decode_ima_aud(&two, 22_050, 0x02).expect("two");
    assert_eq!(a.samples, b.samples);
}
