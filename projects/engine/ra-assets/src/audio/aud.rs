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
    let mut state = ima_adpcm::ImaState::new();
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
        // IMA 预测器跨 DEAF 块延续；每块清零会导致能量塌缩、几乎听不见。
        ima_adpcm::decode_nibble_stream(&data[offset..end], &mut state, &mut samples);
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

    #[test]
    fn carries_ima_state_across_deaf_chunks() {
        // 两块各 1 字节载荷；连续解码应与单块 2 字节一致。
        fn chunk(payload: &[u8]) -> Vec<u8> {
            let mut c = Vec::new();
            c.extend_from_slice(&(payload.len() as u16).to_le_bytes());
            c.extend_from_slice(&((payload.len() * 4) as u16).to_le_bytes());
            c.extend_from_slice(&CHUNK_MAGIC.to_le_bytes());
            c.extend_from_slice(payload);
            c
        }
        let mut hdr = vec![0u8; 12];
        hdr[0..2].copy_from_slice(&22_050u16.to_le_bytes());
        hdr[10] = 0x02;
        hdr[11] = FORMAT_IMA;
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
}
