//! Microsoft IMA ADPCM 块解码（标准 WAV/AUDIO.BAG 共用）。

/// IMA 步长表（89 项，公开标准）。
const STEP_TABLE: [i32; 89] = [
    7, 8, 9, 10, 11, 12, 13, 14, 16, 17, 19, 21, 23, 25, 28, 31, 34, 37, 41, 45, 50, 55, 60, 66, 73,
    80, 88, 97, 107, 118, 130, 143, 157, 173, 190, 209, 230, 253, 279, 307, 337, 371, 408, 449, 494,
    544, 598, 658, 724, 796, 876, 963, 1060, 1166, 1282, 1411, 1552, 1707, 1878, 2066, 2272, 2499,
    2749, 3024, 3327, 3660, 4026, 4428, 4871, 5358, 5894, 6484, 7132, 7845, 8630, 9493, 10442, 11487,
    12635, 13899, 15289, 16818, 18500, 20350, 22385, 24623, 27086, 29794, 32767,
];

/// 步进索引调整（nibble 低 3 位）。
const INDEX_ADJUST: [i32; 8] = [-1, -1, -1, -1, 2, 4, 6, 8];

const MAX_STEP_INDEX: i32 = 88;
const PREAMBLE_PER_CH: usize = 4;
const GROUP_PER_CH: usize = 4;

/// 供 `.aud` 等多块 nibble 流复用的 IMA 状态。
pub struct ImaState {
    predicted: i32,
    index: i32,
}

impl ImaState {
    /// 零预测器、步长索引 0。
    pub fn new() -> Self {
        Self {
            predicted: 0,
            index: 0,
        }
    }

    fn set(&mut self, predicted: i32, index: i32) {
        self.predicted = predicted;
        self.index = index.clamp(0, MAX_STEP_INDEX);
    }

    fn decode_nibble(&mut self, nibble: u8) -> i16 {
        let step = STEP_TABLE[self.index as usize];
        let code = nibble & 0x07;
        let mut diff = step >> 3;
        if code & 0x04 != 0 {
            diff += step;
        }
        if code & 0x02 != 0 {
            diff += step >> 1;
        }
        if code & 0x01 != 0 {
            diff += step >> 2;
        }
        if nibble & 0x08 != 0 {
            self.predicted -= diff;
        } else {
            self.predicted += diff;
        }
        self.predicted = self.predicted.clamp(-32768, 32767);
        self.index += INDEX_ADJUST[code as usize];
        self.index = self.index.clamp(0, MAX_STEP_INDEX);
        self.predicted as i16
    }
}

/// 连续 nibble 流解码（Westwood `.aud` DEAF 块载荷，无块前导）。
///
/// 调用方应跨多个 DEAF 块复用同一 `state`（IMA 预测器不能每块清零）。
pub fn decode_nibble_stream(data: &[u8], state: &mut ImaState, out: &mut Vec<i16>) {
    for &byte in data {
        out.push(state.decode_nibble(byte & 0x0F));
        out.push(state.decode_nibble((byte >> 4) & 0x0F));
    }
}

/// 按块对齐解码 IMA ADPCM → 交错 `i16`。
///
/// `block_align` 为每块字节数（`nBlockAlign`）。为 0 时按声道取零售常见默认
///（单声道 512 / 立体声 1024）。
pub fn decode_blocks(data: &[u8], channels: u16, block_align: u32) -> Vec<i16> {
    let channels = channels.max(1) as usize;
    let preamble = PREAMBLE_PER_CH * channels;
    let group = GROUP_PER_CH * channels;
    let mut block_align = block_align as usize;
    if block_align < preamble {
        block_align = if channels == 1 { 512 } else { 1024 };
    }
    if block_align < preamble {
        return Vec::new();
    }

    let mut out = Vec::with_capacity(data.len() * 2 + channels);
    let mut states: Vec<ImaState> = (0..channels).map(|_| ImaState::new()).collect();
    let mut pos = 0usize;

    while pos + preamble <= data.len() {
        let end = (pos + block_align).min(data.len());
        let block = &data[pos..end];
        if block.len() < preamble {
            break;
        }

        let mut ok = true;
        for ch in 0..channels {
            let o = ch * PREAMBLE_PER_CH;
            let predicted = i16::from_le_bytes([block[o], block[o + 1]]) as i32;
            let index = block[o + 2] as i32;
            let reserved = block[o + 3];
            if index > MAX_STEP_INDEX || reserved != 0 {
                ok = false;
                break;
            }
            states[ch].set(predicted, index);
        }
        if !ok {
            break;
        }

        // 块首帧：各声道预测值。
        for ch in 0..channels {
            out.push(states[ch].predicted as i16);
        }

        let mut p = preamble;
        while p + group <= block.len() {
            // 每声道 4 字节 → 8 个 nibble → 8 帧。
            let mut frame_samples = [[0i16; 8]; 2];
            for ch in 0..channels {
                let slice = &block[p + ch * GROUP_PER_CH..p + (ch + 1) * GROUP_PER_CH];
                let mut n = 0usize;
                for &byte in slice {
                    frame_samples[ch][n] = states[ch].decode_nibble(byte & 0x0F);
                    n += 1;
                    frame_samples[ch][n] = states[ch].decode_nibble((byte >> 4) & 0x0F);
                    n += 1;
                }
            }
            for i in 0..8 {
                for ch in 0..channels {
                    out.push(frame_samples[ch][i]);
                }
            }
            p += group;
        }

        pos += block_align;
    }

    out
}

#[cfg(test)]
mod tests {
    use super::*;

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
}
