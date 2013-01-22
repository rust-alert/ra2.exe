//! MIX 档案头探测（完整索引读取后续再补）。

use ra_types::{RaError, RaResult};

#[derive(Debug, Clone)]
pub struct MixEntry {
    pub id: u32,
    pub offset: u32,
    pub size: u32,
}

#[derive(Debug, Clone)]
pub struct MixArchive {
    pub flags: u32,
    pub entries: Vec<MixEntry>,
    pub body_offset: usize,
    pub data: Vec<u8>,
}

impl MixArchive {
    /// 解析未加密的经典头 MIX（足以校验文件可读）。
    pub fn parse(data: Vec<u8>) -> RaResult<Self> {
        if data.len() < 10 {
            return Err(RaError::Parse("mix 过小".into()));
        }
        let flags = u32::from_le_bytes(data[0..4].try_into().unwrap());
        if flags != 0 {
            return Err(RaError::Parse(format!("暂不支持的 mix flags {flags:#x}")));
        }
        let count = u16::from_le_bytes(data[4..6].try_into().unwrap()) as usize;
        let _size = u32::from_le_bytes(data[6..10].try_into().unwrap());
        let header_end = 10 + count * 12;
        if data.len() < header_end {
            return Err(RaError::Parse("mix 头截断".into()));
        }
        let mut entries = Vec::with_capacity(count);
        for i in 0..count {
            let o = 10 + i * 12;
            let id = u32::from_le_bytes(data[o..o + 4].try_into().unwrap());
            let offset = u32::from_le_bytes(data[o + 4..o + 8].try_into().unwrap());
            let size = u32::from_le_bytes(data[o + 8..o + 12].try_into().unwrap());
            entries.push(MixEntry { id, offset, size });
        }
        Ok(Self {
            flags,
            entries,
            body_offset: header_end,
            data,
        })
    }

    pub fn entry_count(&self) -> usize {
        self.entries.len()
    }
}
