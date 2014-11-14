//! 集成测试：原 `src/audio/mod.rs` 内联测试迁出。

use ra_assets::decode_wav_pcm;

fn build_pcm16_wav(channels: u16, rate: u32, samples: &[i16]) -> Vec<u8> {
    let data_bytes = samples.len() * 2;
    let fmt_size = 16u32;
    let riff_size = 4 + 8 + fmt_size + 8 + data_bytes as u32;
    let mut out = Vec::new();
    out.extend_from_slice(b"RIFF");
    out.extend_from_slice(&riff_size.to_le_bytes());
    out.extend_from_slice(b"WAVE");
    out.extend_from_slice(b"fmt ");
    out.extend_from_slice(&fmt_size.to_le_bytes());
    out.extend_from_slice(&1u16.to_le_bytes());
    out.extend_from_slice(&channels.to_le_bytes());
    out.extend_from_slice(&rate.to_le_bytes());
    let byte_rate = rate * u32::from(channels) * 2;
    out.extend_from_slice(&byte_rate.to_le_bytes());
    let block_align = channels * 2;
    out.extend_from_slice(&block_align.to_le_bytes());
    out.extend_from_slice(&16u16.to_le_bytes());
    out.extend_from_slice(b"data");
    out.extend_from_slice(&(data_bytes as u32).to_le_bytes());
    for s in samples {
        out.extend_from_slice(&s.to_le_bytes());
    }
    out
}

#[test]
fn symphonia_decodes_mono_pcm16_wav() {
    let samples = [0i16, 1000, -1000, 32767];
    let wav = build_pcm16_wav(1, 22050, &samples);
    let pcm = decode_wav_pcm(&wav).expect("wav");
    assert_eq!(pcm.sample_rate, 22050);
    assert_eq!(pcm.channels, 1);
    assert_eq!(pcm.samples, samples);
}

#[test]
fn reject_garbage() {
    assert!(decode_wav_pcm(b"not a wav").is_err());
}
