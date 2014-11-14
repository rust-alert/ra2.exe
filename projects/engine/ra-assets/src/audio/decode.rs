//! 用 Symphonia 把内存中的音频字节解成交错 PCM16。
//!
//! 覆盖主题曲等标准容器（首期启用 `wav` + `pcm`）。Westwood `.aud` /
//! `audio.bag` 专有载荷不在此路径，另接。

use std::io::Cursor;

use symphonia::core::{
    codecs::{
        CodecParameters,
        audio::{AudioCodecParameters, AudioDecoderOptions, CODEC_ID_NULL_AUDIO},
    },
    errors::Error as SymError,
    formats::{FormatOptions, TrackType, probe::Hint},
    io::MediaSourceStream,
    meta::MetadataOptions,
};

use super::{PcmAudio, WavError};

/// 探测并解码整段字节（无文件路径，适合 MIX 读出的缓冲）。
pub fn decode_audio_bytes(data: &[u8], hint_ext: Option<&str>) -> Result<PcmAudio, WavError> {
    // 主题曲多为 Microsoft IMA ADPCM WAV（格式标签 17）；Symphonia 默认未编入该解码器。
    if let Some(riff) = super::wav_riff::try_decode_riff_wave(data) {
        return riff;
    }
    // 中文盘菜单曲常见为 `local.mix` 内 `intro.aud`（Westwood AUD / 格式 99）。
    if let Some(aud) = super::aud::try_decode_aud(data) {
        return aud;
    }
    // 扩展名提示为 aud 但仍未识别时，给出更明确错误。
    if hint_ext.is_some_and(|e| e.eq_ignore_ascii_case("aud")) {
        return Err(WavError::UnsupportedWavFormat(0));
    }

    let cursor = Cursor::new(data.to_vec());
    let mss = MediaSourceStream::new(Box::new(cursor), Default::default());

    let mut hint = Hint::new();
    if let Some(ext) = hint_ext {
        hint.with_extension(ext.trim_start_matches('.'));
    }

    let mut format =
        symphonia::default::get_probe().probe(&hint, mss, FormatOptions::default(), MetadataOptions::default()).map_err(map_sym)?;

    let track = format.default_track(TrackType::Audio).ok_or(WavError::NoAudioTrack)?.clone();

    let audio_params = audio_codec_params(&track.codec_params)?;
    let sample_rate = audio_params.sample_rate.ok_or(WavError::MissingSampleRate)?;
    let channels = audio_params.channels.as_ref().map(|c| c.count() as u16).ok_or(WavError::MissingChannels)?;
    if channels == 0 || channels > 2 {
        return Err(WavError::BadChannels(channels));
    }

    let mut decoder = symphonia::default::get_codecs().make_audio_decoder(audio_params, &AudioDecoderOptions::default()).map_err(map_sym)?;

    let track_id = track.id;
    let mut samples: Vec<i16> = Vec::new();
    let mut chunk: Vec<i16> = Vec::new();

    loop {
        let packet = match format.next_packet() {
            Ok(Some(packet)) => packet,
            Ok(None) => break,
            Err(SymError::ResetRequired) => {
                // 内存缓冲整段解码极少遇到轨表变更；遇此直接失败即可。
                return Err(map_sym(SymError::ResetRequired));
            }
            Err(e) => return Err(map_sym(e)),
        };
        if packet.track_id != track_id {
            continue;
        }

        let decoded = match decoder.decode(&packet) {
            Ok(audio) => audio,
            Err(SymError::DecodeError(_)) => continue,
            Err(SymError::ResetRequired) => {
                decoder.reset();
                continue;
            }
            Err(e) => return Err(map_sym(e)),
        };

        // `copy_to_vec_interleaved` 会按本包长度 resize，不能直接往总缓冲写。
        decoded.copy_to_vec_interleaved(&mut chunk);
        samples.extend_from_slice(&chunk);
    }

    if samples.is_empty() {
        return Err(WavError::EmptyDecode);
    }

    Ok(PcmAudio { sample_rate, channels, samples })
}

/// 按 WAV 扩展名提示解码。
pub fn decode_wav_pcm(data: &[u8]) -> Result<PcmAudio, WavError> {
    decode_audio_bytes(data, Some("wav"))
}

fn audio_codec_params(params: &Option<CodecParameters>) -> Result<&AudioCodecParameters, WavError> {
    match params {
        Some(CodecParameters::Audio(audio)) if audio.codec != CODEC_ID_NULL_AUDIO => Ok(audio),
        _ => Err(WavError::NoAudioTrack),
    }
}

fn map_sym(err: SymError) -> WavError {
    WavError::Symphonia(err.to_string())
}
