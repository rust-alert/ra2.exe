//! Westwood `.aud`：12 字节头 + `0xDEAF` 分块 IMA ADPCM（格式 99）。
//!
//! 公开格式说明见社区 AUD 文档（SamplesPerSec / Size / OutSize / Flags / Typ）。
//! 中文盘常把菜单曲以 `intro.aud` 放在 `local.mix`，而非 `theme.mix` 内的 `*.wav`。

use super::ima_adpcm;
use super::{PcmAudio, WavError};

const HEADER_SIZE: usize = 12;
const CHUNK_HEADER_SIZE: usize = 8;
const CHUNK_MAGIC: u32 = 0x0000_DEAF;
const FORMAT_IMA: u8 = 99;

/// 若像 `.aud`（采样率合理且格式为 99），则解码；否则 `None` 交给调用方回退。
pub(crate) fn try_decode_aud(data: &[u8]) -> Option<Result<PcmAudio, WavError>> {
    if data.len() < HEADER_SIZE {
        return None;
    }
    let sample_rate = u16::from_le_bytes([data[0], data[1]]);
    let flags = data[10];
    let format = data[11];
    if !(8_000..=48_000).contains(&sample_rate) {
        return None;
    }
    if format != FORMAT_IMA {
        // 非 IMA 的 AUD 暂不宣称可解，避免误吃别的载荷。
        return None;
    }
    Some(decode_ima_aud(data, sample_rate, flags))
}

fn decode_ima_aud(data: &[u8], sample_rate: u16, flags: u8) -> Result<PcmAudio, WavError> {
    let channels: u16 = if flags & 0x01 != 0 { 2 } else { 1 };
    if channels != 1 {
        // 零售菜单 intro.aud 为单声道；立体声 AUD 需交错策略，首期只接 mono。
        return Err(WavError::BadChannels(channels));
    }

    let mut samples: Vec<i16> = Vec::new();
    let mut offset = HEADER_SIZE;
    while offset + CHUNK_HEADER_SIZE <= data.len() {
        let compressed_size = u16::from_le_bytes([data[offset], data[offset + 1]]) as usize;
        let magic = u32::from_le_bytes([
            data[offset + 4],
            data[offset + 5],
            data[offset + 6],
            data[offset + 7],
        ]);
        if magic != CHUNK_MAGIC {
            break;
        }
        offset += CHUNK_HEADER_SIZE;
        let end = (offset + compressed_size).min(data.len());
        ima_adpcm::decode_nibble_stream(&data[offset..end], &mut samples);
        offset = end;
    }

    if samples.is_empty() {
        return Err(WavError::EmptyDecode);
    }
    Ok(PcmAudio {
        sample_rate: u32::from(sample_rate),
        channels: 1,
        samples,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_short_and_non_aud() {
        assert!(try_decode_aud(&[0u8; 8]).is_none());
        assert!(try_decode_aud(b"RIFF........").is_none());
    }
}
