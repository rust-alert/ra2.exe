//! 用 Symphonia 把内存中的音频字节解成交错 PCM16。
//!
//! 覆盖主题曲等标准容器（首期启用 `wav` + `pcm`）。Westwood `.aud` /
//! `audio.bag` 专有载荷不在此路径，另接。

use std::io::Cursor;

use symphonia::core::audio::SampleBuffer;
use symphonia::core::codecs::{CODEC_TYPE_NULL, DecoderOptions};
use symphonia::core::errors::Error as SymError;
use symphonia::core::formats::FormatOptions;
use symphonia::core::io::MediaSourceStream;
use symphonia::core::meta::MetadataOptions;
use symphonia::core::probe::Hint;

use super::{PcmAudio, WavError};

/// 探测并解码整段字节（无文件路径，适合 MIX 读出的缓冲）。
pub fn decode_audio_bytes(data: &[u8], hint_ext: Option<&str>) -> Result<PcmAudio, WavError> {
    // 主题曲多为 Microsoft IMA ADPCM WAV（格式标签 17）；Symphonia 默认未编入该解码器。
    if let Some(riff) = super::wav_riff::try_decode_riff_wave(data) {
        return riff;
    }

    let cursor = Cursor::new(data.to_vec());
    let mss = MediaSourceStream::new(Box::new(cursor), Default::default());

    let mut hint = Hint::new();
    if let Some(ext) = hint_ext {
        hint.with_extension(ext.trim_start_matches('.'));
    }

    let probed = symphonia::default::get_probe()
        .format(&hint, mss, &FormatOptions::default(), &MetadataOptions::default())
        .map_err(map_sym)?;

    let mut format = probed.format;
    let track = format
        .tracks()
        .iter()
        .find(|t| t.codec_params.codec != CODEC_TYPE_NULL)
        .ok_or(WavError::NoAudioTrack)?
        .clone();

    let sample_rate = track.codec_params.sample_rate.ok_or(WavError::MissingSampleRate)?;
    let channels = track
        .codec_params
        .channels
        .map(|c| c.count() as u16)
        .ok_or(WavError::MissingChannels)?;
    if channels == 0 || channels > 2 {
        return Err(WavError::BadChannels(channels));
    }

    let mut decoder = symphonia::default::get_codecs()
        .make(&track.codec_params, &DecoderOptions::default())
        .map_err(map_sym)?;

    let track_id = track.id;
    let mut samples: Vec<i16> = Vec::new();
    let mut sample_buf: Option<SampleBuffer<i16>> = None;

    loop {
        let packet = match format.next_packet() {
            Ok(packet) => packet,
            Err(SymError::IoError(_)) => break,
            Err(SymError::ResetRequired) => {
                decoder.reset();
                continue;
            }
            Err(e) => return Err(map_sym(e)),
        };
        if packet.track_id() != track_id {
            continue;
        }

        let decoded = match decoder.decode(&packet) {
            Ok(audio) => audio,
            Err(SymError::DecodeError(_)) => continue,
            Err(e) => return Err(map_sym(e)),
        };

        if sample_buf.is_none() {
            sample_buf = Some(SampleBuffer::new(decoded.capacity() as u64, *decoded.spec()));
        }
        let buf = sample_buf.as_mut().unwrap();
        buf.copy_interleaved_ref(decoded);
        samples.extend_from_slice(buf.samples());
    }

    if samples.is_empty() {
        return Err(WavError::EmptyDecode);
    }

    Ok(PcmAudio {
        sample_rate,
        channels,
        samples,
    })
}

/// 按 WAV 扩展名提示解码。
pub fn decode_wav_pcm(data: &[u8]) -> Result<PcmAudio, WavError> {
    decode_audio_bytes(data, Some("wav"))
}

fn map_sym(err: SymError) -> WavError {
    WavError::Symphonia(err.to_string())
}
