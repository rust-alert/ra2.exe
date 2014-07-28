//! 音频解码：RIFF / Westwood AUD / Symphonia；`audio.bag` 走专有索引。

mod aud;
mod bag;
mod decode;
mod ima_adpcm;
mod wav_riff;

pub use bag::{AudioBagEntry, AudioIndex};
pub use decode::{decode_audio_bytes, decode_wav_pcm};

/// 解码后的交错 PCM16。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PcmAudio {
    /// 采样率（Hz）。
    pub sample_rate: u32,
    /// 声道数（1 或 2）。
    pub channels: u16,
    /// 交错有符号 16-bit 采样。
    pub samples: Vec<i16>,
}

/// 音频解码失败。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WavError {
    /// Symphonia / 探测失败。
    Symphonia(String),
    /// 无可用音轨。
    NoAudioTrack,
    /// 缺采样率。
    MissingSampleRate,
    /// 缺声道信息。
    MissingChannels,
    /// 声道数非法。
    BadChannels(u16),
    /// 解出 0 采样。
    EmptyDecode,
    /// 不支持的 WAV 编码（如未实现的压缩格式标签）。
    UnsupportedWavFormat(u16),
}

impl std::fmt::Display for WavError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Symphonia(msg) => write!(f, "symphonia: {msg}"),
            Self::NoAudioTrack => write!(f, "无音频轨"),
            Self::MissingSampleRate => write!(f, "缺采样率"),
            Self::MissingChannels => write!(f, "缺声道信息"),
            Self::BadChannels(n) => write!(f, "非法声道数 {n}"),
            Self::EmptyDecode => write!(f, "解码结果为空"),
            Self::UnsupportedWavFormat(tag) => write!(f, "不支持的 WAV 格式标签 {tag}"),
        }
    }
}

impl std::error::Error for WavError {}

#[cfg(test)]
mod tests {
    use super::*;

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
}
