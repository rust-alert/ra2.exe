//! 集成测试：原 `src/audio/wav_riff.rs` 内联测试迁出。

use ra_assets::audio::wav_riff::try_decode_riff_wave;

fn pcm16_wav(channels: u16, rate: u32, samples: &[i16]) -> Vec<u8> {
    let data_bytes = samples.len() * 2;
    let mut out = Vec::new();
    out.extend_from_slice(b"RIFF");
    out.extend_from_slice(&(36 + data_bytes as u32).to_le_bytes());
    out.extend_from_slice(b"WAVE");
    out.extend_from_slice(b"fmt ");
    out.extend_from_slice(&16u32.to_le_bytes());
    out.extend_from_slice(&1u16.to_le_bytes());
    out.extend_from_slice(&channels.to_le_bytes());
    out.extend_from_slice(&rate.to_le_bytes());
    let byte_rate = rate * u32::from(channels) * 2;
    out.extend_from_slice(&byte_rate.to_le_bytes());
    out.extend_from_slice(&(channels * 2).to_le_bytes());
    out.extend_from_slice(&16u16.to_le_bytes());
    out.extend_from_slice(b"data");
    out.extend_from_slice(&(data_bytes as u32).to_le_bytes());
    for s in samples {
        out.extend_from_slice(&s.to_le_bytes());
    }
    out
}

#[test]
fn decodes_pcm16_riff() {
    let samples = [0i16, 1000, -1000];
    let wav = pcm16_wav(1, 22050, &samples);
    let pcm = try_decode_riff_wave(&wav).unwrap().unwrap();
    assert_eq!(pcm.samples, samples);
    assert_eq!(pcm.sample_rate, 22050);
}
