//! 音频解码：RIFF / Westwood AUD / Symphonia；`audio.bag` 走专有索引。

pub mod aud;
mod bag;
mod decode;
pub mod ima_adpcm;
pub mod wav_riff;

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
