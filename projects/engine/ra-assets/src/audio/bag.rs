//! `audio.idx` / `audio.bag`：RA2 音效包索引与载荷切片。

use super::ima_adpcm;
use super::PcmAudio;

const IDX_HEADER: usize = 12;
const ENTRY_V1: usize = 32;
const ENTRY_V2: usize = 36;

const FLAG_STEREO: u32 = 0x01;
const FLAG_16BIT: u32 = 0x04;
const FLAG_IMA: u32 = 0x08;

/// `audio.idx` 中一条记录。
#[derive(Debug, Clone)]
pub struct AudioBagEntry {
    /// 名称（大写，最多 15 字符）。
    pub name: String,
    /// 在 bag 中的字节偏移。
    pub offset: u32,
    /// 载荷字节数。
    pub size: u32,
    /// 采样率。
    pub sample_rate: u32,
    /// 标志位。
    pub flags: u32,
    /// IMA 块对齐（v2）；v1 为 0。
    pub chunk_size: u32,
}

impl AudioBagEntry {
    /// 立体声。
    pub fn is_stereo(&self) -> bool {
        self.flags & FLAG_STEREO != 0
    }

    /// 16-bit PCM。
    pub fn is_16bit(&self) -> bool {
        self.flags & FLAG_16BIT != 0
    }

    /// IMA ADPCM。
    pub fn is_ima_adpcm(&self) -> bool {
        self.flags & FLAG_IMA != 0
    }

    /// 声道数。
    pub fn channels(&self) -> u16 {
        if self.is_stereo() { 2 } else { 1 }
    }
}

/// 已加载的 idx+bag。
#[derive(Debug, Clone)]
pub struct AudioIndex {
    entries: Vec<AudioBagEntry>,
    bag: Vec<u8>,
}

impl AudioIndex {
    /// 解析 idx 字节并持有 bag 缓冲。
    pub fn parse(idx: &[u8], bag: Vec<u8>) -> Option<Self> {
        if idx.len() < IDX_HEADER {
            return None;
        }
        let version = u32::from_le_bytes(idx[4..8].try_into().ok()?);
        let count = u32::from_le_bytes(idx[8..12].try_into().ok()?) as usize;
        let entry_size = match version {
            1 => ENTRY_V1,
            2 => ENTRY_V2,
            _ => return None,
        };
        let need = IDX_HEADER.checked_add(count.checked_mul(entry_size)?)?;
        if idx.len() < need {
            return None;
        }

        let mut entries = Vec::with_capacity(count);
        let mut off = IDX_HEADER;
        for _ in 0..count {
            let raw = &idx[off..off + entry_size];
            let name_bytes = &raw[0..16];
            let end = name_bytes.iter().position(|&b| b == 0).unwrap_or(16);
            let name = String::from_utf8_lossy(&name_bytes[..end]).to_ascii_uppercase();
            let offset = u32::from_le_bytes(raw[16..20].try_into().ok()?);
            let size = u32::from_le_bytes(raw[20..24].try_into().ok()?);
            let sample_rate = u32::from_le_bytes(raw[24..28].try_into().ok()?);
            let flags = u32::from_le_bytes(raw[28..32].try_into().ok()?);
            let chunk_size = if entry_size == ENTRY_V2 {
                u32::from_le_bytes(raw[32..36].try_into().ok()?)
            } else {
                0
            };
            entries.push(AudioBagEntry {
                name,
                offset,
                size,
                sample_rate,
                flags,
                chunk_size,
            });
            off += entry_size;
        }

        Some(Self { entries, bag })
    }

    /// 条目数。
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// 是否为空。
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// 所有条目名（大写，按 idx 顺序）。
    pub fn names(&self) -> impl Iterator<Item = &str> + '_ {
        self.entries.iter().map(|e| e.name.as_str())
    }

    /// 按名查找（大小写不敏感；过长名按 idx 字段宽度截断再试）。
    pub fn get(&self, name: &str) -> Option<&AudioBagEntry> {
        let key = name.to_ascii_uppercase();
        if let Some(e) = self.entries.iter().find(|e| e.name == key) {
            return Some(e);
        }
        // idx 名槽 16 字节，常含结尾 0，有效最长约 15。
        let trunc15: String = key.chars().take(15).collect();
        if trunc15 != key {
            if let Some(e) = self.entries.iter().find(|e| e.name == trunc15) {
                return Some(e);
            }
        }
        let trunc16: String = key.chars().take(16).collect();
        if trunc16 != key && trunc16 != trunc15 {
            self.entries.iter().find(|e| e.name == trunc16)
        } else {
            None
        }
    }

    /// 解码指定条目为 PCM16。
    pub fn decode(&self, name: &str) -> Option<PcmAudio> {
        let entry = self.get(name)?;
        let start = entry.offset as usize;
        let end = start.checked_add(entry.size as usize)?;
        let data = self.bag.get(start..end)?;
        decode_entry(entry, data)
    }
}

fn decode_entry(entry: &AudioBagEntry, data: &[u8]) -> Option<PcmAudio> {
    if data.is_empty() {
        return None;
    }
    let samples = if entry.is_ima_adpcm() {
        ima_adpcm::decode_blocks(data, entry.channels(), entry.chunk_size)
    } else if entry.is_16bit() {
        if data.len() % 2 != 0 {
            return None;
        }
        data.chunks_exact(2)
            .map(|c| i16::from_le_bytes([c[0], c[1]]))
            .collect()
    } else {
        data.iter().map(|&b| ((b as i16) - 128) << 8).collect()
    };
    if samples.is_empty() {
        return None;
    }
    Some(PcmAudio {
        sample_rate: entry.sample_rate,
        channels: entry.channels(),
        samples,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn build_idx_v1(entries: &[(&str, u32, u32, u32, u32)]) -> Vec<u8> {
        let mut idx = Vec::new();
        idx.extend_from_slice(&0u32.to_le_bytes());
        idx.extend_from_slice(&1u32.to_le_bytes());
        idx.extend_from_slice(&(entries.len() as u32).to_le_bytes());
        for &(name, offset, size, rate, flags) in entries {
            let mut name_buf = [0u8; 16];
            let nb = name.as_bytes();
            let n = nb.len().min(15);
            name_buf[..n].copy_from_slice(&nb[..n]);
            idx.extend_from_slice(&name_buf);
            idx.extend_from_slice(&offset.to_le_bytes());
            idx.extend_from_slice(&size.to_le_bytes());
            idx.extend_from_slice(&rate.to_le_bytes());
            idx.extend_from_slice(&flags.to_le_bytes());
        }
        idx
    }

    #[test]
    fn parse_and_decode_pcm16() {
        let idx = build_idx_v1(&[("CLICK", 0, 4, 22050, FLAG_16BIT)]);
        let bag = {
            let mut b = Vec::new();
            b.extend_from_slice(&1000i16.to_le_bytes());
            b.extend_from_slice(&(-1000i16).to_le_bytes());
            b
        };
        let index = AudioIndex::parse(&idx, bag).unwrap();
        assert_eq!(index.len(), 1);
        let pcm = index.decode("click").unwrap();
        assert_eq!(pcm.sample_rate, 22050);
        assert_eq!(pcm.samples, vec![1000, -1000]);
    }

    #[test]
    fn reject_bad_version() {
        let mut idx = build_idx_v1(&[]);
        idx[4..8].copy_from_slice(&99u32.to_le_bytes());
        assert!(AudioIndex::parse(&idx, vec![]).is_none());
    }
}
