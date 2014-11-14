//! RIFF WAVE 轻量解析：PCM16 与 Microsoft IMA ADPCM（主题曲常用）。

use super::{PcmAudio, WavError, ima_adpcm};

const WAVE_FORMAT_PCM: u16 = 1;
const WAVE_FORMAT_IMA_ADPCM: u16 = 17;

#[derive(Debug, Clone)]
struct WavStream<'a> {
    format: u16,
    channels: u16,
    sample_rate: u32,
    block_align: u16,
    bits_per_sample: u16,
    data: &'a [u8],
}

/// 若是 RIFF/WAVE，走自研解码（含 IMA ADPCM）；否则交给调用方回退。
pub fn try_decode_riff_wave(data: &[u8]) -> Option<Result<PcmAudio, WavError>> {
    let stream = parse_riff_wave(data)?;
    Some(decode_stream(stream))
}

fn parse_riff_wave(data: &[u8]) -> Option<WavStream<'_>> {
    if data.len() < 12 || &data[0..4] != b"RIFF" || &data[8..12] != b"WAVE" {
        return None;
    }
    let mut format = None;
    let mut channels = None;
    let mut sample_rate = None;
    let mut block_align = None;
    let mut bits_per_sample = None;
    let mut pcm_data = None;

    let mut off = 12usize;
    while off + 8 <= data.len() {
        let id = &data[off..off + 4];
        let size = u32::from_le_bytes(data[off + 4..off + 8].try_into().ok()?) as usize;
        let body = off + 8;
        let end = body.checked_add(size)?;
        if end > data.len() {
            break;
        }
        match id {
            b"fmt " if size >= 16 => {
                format = Some(u16::from_le_bytes(data[body..body + 2].try_into().ok()?));
                channels = Some(u16::from_le_bytes(data[body + 2..body + 4].try_into().ok()?));
                sample_rate = Some(u32::from_le_bytes(data[body + 4..body + 8].try_into().ok()?));
                block_align = Some(u16::from_le_bytes(data[body + 12..body + 14].try_into().ok()?));
                bits_per_sample = Some(u16::from_le_bytes(data[body + 14..body + 16].try_into().ok()?));
            }
            b"data" => {
                pcm_data = Some(&data[body..end]);
            }
            _ => {}
        }
        // 奇数长度 chunk 后跟填充字节。
        off = end + (size & 1);
    }

    Some(WavStream {
        format: format?,
        channels: channels?,
        sample_rate: sample_rate?,
        block_align: block_align?,
        bits_per_sample: bits_per_sample?,
        data: pcm_data?,
    })
}

fn decode_stream(stream: WavStream<'_>) -> Result<PcmAudio, WavError> {
    if stream.channels == 0 || stream.channels > 2 {
        return Err(WavError::BadChannels(stream.channels));
    }
    if stream.sample_rate == 0 {
        return Err(WavError::MissingSampleRate);
    }
    let samples = match stream.format {
        WAVE_FORMAT_PCM => decode_pcm16(stream.data, stream.channels, stream.bits_per_sample)?,
        WAVE_FORMAT_IMA_ADPCM => {
            let samples = ima_adpcm::decode_blocks(stream.data, stream.channels, u32::from(stream.block_align));
            if samples.is_empty() {
                return Err(WavError::EmptyDecode);
            }
            samples
        }
        other => {
            return Err(WavError::UnsupportedWavFormat(other));
        }
    };
    Ok(PcmAudio { sample_rate: stream.sample_rate, channels: stream.channels, samples })
}

fn decode_pcm16(data: &[u8], _channels: u16, bits: u16) -> Result<Vec<i16>, WavError> {
    match bits {
        16 => {
            if data.len() % 2 != 0 {
                return Err(WavError::EmptyDecode);
            }
            Ok(data.chunks_exact(2).map(|c| i16::from_le_bytes([c[0], c[1]])).collect())
        }
        8 => Ok(data.iter().map(|&b| ((b as i16) - 128) << 8).collect()),
        _ => Err(WavError::UnsupportedWavFormat(WAVE_FORMAT_PCM)),
    }
}
