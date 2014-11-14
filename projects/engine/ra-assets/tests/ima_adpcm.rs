//! 集成测试：原 `src/audio/ima_adpcm.rs` 内联测试迁出。

use ra_assets::audio::ima_adpcm::decode_blocks;

#[test]
fn empty_input_yields_empty() {
    assert!(decode_blocks(&[], 1, 512).is_empty());
}

#[test]
fn mono_preamble_only_emits_predictor() {
    // predictor=1000, index=0, reserved=0，无载荷。
    let block = {
        let mut b = vec![0u8; 512];
        b[0..2].copy_from_slice(&1000i16.to_le_bytes());
        b[2] = 0;
        b[3] = 0;
        b
    };
    let out = decode_blocks(&block, 1, 512);
    assert_eq!(out.first().copied(), Some(1000));
}
