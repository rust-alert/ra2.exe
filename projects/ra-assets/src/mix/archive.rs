//! MIX 档案：旧头 / 新格式（可加密索引）。

use ra_types::{RaError, RaResult};

use super::{
    crypto::{BLOWFISH_BLOCK_SIZE, RSA_KEY_BLOCK_SIZE},
    hash::mix_hash,
};

/// 新格式：索引经 Blowfish 加密。
const FLAG_ENCRYPTED: u16 = 0x0002;
const NEW_FORMAT_HEADER_SIZE: usize = 4;
const INDEX_ENTRY_SIZE: usize = 12;
const INDEX_HEADER_SIZE: usize = 6;

/// MIX 索引中的单条文件记录。
#[derive(Debug, Clone)]
pub struct MixEntry {
    /// 文件名哈希（`mix_hash` 结果）。
    pub id: i32,
    /// 相对正文起点的字节偏移。
    pub offset: u32,
    /// 文件字节长度。
    pub size: u32,
}

/// 已解析的 MIX 档案（持有完整文件字节）。
#[derive(Debug, Clone)]
pub struct MixArchive {
    /// 新格式标志位；旧头恒为 0。
    pub flags: u16,
    entries: Vec<MixEntry>,
    body_offset: usize,
    data: Vec<u8>,
}

impl MixArchive {
    /// 解析 MIX 字节（自动区分旧头与新格式）。
    pub fn parse(data: Vec<u8>) -> RaResult<Self> {
        if data.len() < 4 {
            return Err(RaError::Parse("mix 过小".into()));
        }

        let first_word = read_u16(&data, 0);
        let (flags, mut entries, body_offset) = if first_word == 0 {
            let flags = read_u16(&data, 2);
            let (entries, body_offset) =
                if (flags & FLAG_ENCRYPTED) != 0 { parse_encrypted_new_format(&data)? } else { parse_unencrypted_new_format(&data)? };
            (flags, entries, body_offset)
        }
        else {
            let (entries, body_offset) = parse_old_format(&data, first_word)?;
            (0, entries, body_offset)
        };

        entries.sort_by_key(|e| e.id);
        Ok(Self { flags, entries, body_offset, data })
    }

    /// 索引条目数量。
    pub fn entry_count(&self) -> usize {
        self.entries.len()
    }

    /// 按 id 排序后的条目切片。
    pub fn entries(&self) -> &[MixEntry] {
        &self.entries
    }

    /// 按文件名查找（CRC-32 + Westwood 填充）。
    pub fn get_by_name(&self, name: &str) -> Option<&[u8]> {
        self.get_by_id(mix_hash(name))
    }

    /// 按条目 id 取正文切片。
    pub fn get_by_id(&self, id: i32) -> Option<&[u8]> {
        let index = self.entries.binary_search_by_key(&id, |e| e.id).ok()?;
        let entry = &self.entries[index];
        let start = self.body_offset.checked_add(entry.offset as usize)?;
        let end = start.checked_add(entry.size as usize)?;
        self.data.get(start..end)
    }
}

fn parse_encrypted_new_format(data: &[u8]) -> RaResult<(Vec<MixEntry>, usize)> {
    let rsa_start = NEW_FORMAT_HEADER_SIZE;
    let rsa_end = rsa_start + RSA_KEY_BLOCK_SIZE;
    if data.len() < rsa_end {
        return Err(RaError::Parse("mix 缺少 RSA 密钥块".into()));
    }

    let blowfish_key = super::crypto::extract_blowfish_key(&data[rsa_start..rsa_end])?;
    let encrypted_start = rsa_end;
    if data.len() < encrypted_start + BLOWFISH_BLOCK_SIZE {
        return Err(RaError::Parse("mix 加密索引过小".into()));
    }

    let mut first_block = [0u8; BLOWFISH_BLOCK_SIZE];
    first_block.copy_from_slice(&data[encrypted_start..encrypted_start + BLOWFISH_BLOCK_SIZE]);
    super::crypto::blowfish_decrypt_ecb(&blowfish_key, &mut first_block)?;

    let file_count = read_u16(&first_block, 0) as usize;
    let total_index_bytes = INDEX_HEADER_SIZE + file_count * INDEX_ENTRY_SIZE;
    let encrypted_size = total_index_bytes.div_ceil(BLOWFISH_BLOCK_SIZE) * BLOWFISH_BLOCK_SIZE;

    if data.len() < encrypted_start + encrypted_size {
        return Err(RaError::Parse(format!("mix 加密索引截断: 需要 {} 字节", encrypted_start + encrypted_size)));
    }

    let mut decrypted = data[encrypted_start..encrypted_start + encrypted_size].to_vec();
    super::crypto::blowfish_decrypt_ecb(&blowfish_key, &mut decrypted)?;
    let entries = parse_entries(&decrypted, INDEX_HEADER_SIZE, file_count)?;
    Ok((entries, encrypted_start + encrypted_size))
}

fn parse_unencrypted_new_format(data: &[u8]) -> RaResult<(Vec<MixEntry>, usize)> {
    if data.len() < NEW_FORMAT_HEADER_SIZE + INDEX_HEADER_SIZE {
        return Err(RaError::Parse("mix 新格式头过小".into()));
    }
    let file_count = read_u16(data, NEW_FORMAT_HEADER_SIZE) as usize;
    let index_start = NEW_FORMAT_HEADER_SIZE + INDEX_HEADER_SIZE;
    let index_end = index_start + file_count * INDEX_ENTRY_SIZE;
    if data.len() < index_end {
        return Err(RaError::Parse("mix 新格式索引截断".into()));
    }
    let entries = parse_entries(data, index_start, file_count)?;
    Ok((entries, index_end))
}

fn parse_old_format(data: &[u8], file_count: u16) -> RaResult<(Vec<MixEntry>, usize)> {
    if data.len() < INDEX_HEADER_SIZE {
        return Err(RaError::Parse("mix 旧头过小".into()));
    }
    let index_start = INDEX_HEADER_SIZE;
    let index_end = index_start + file_count as usize * INDEX_ENTRY_SIZE;
    if data.len() < index_end {
        return Err(RaError::Parse("mix 旧头索引截断".into()));
    }
    let entries = parse_entries(data, index_start, file_count as usize)?;
    Ok((entries, index_end))
}

fn parse_entries(data: &[u8], start: usize, file_count: usize) -> RaResult<Vec<MixEntry>> {
    let mut entries = Vec::with_capacity(file_count);
    for i in 0..file_count {
        let o = start + i * INDEX_ENTRY_SIZE;
        if o + INDEX_ENTRY_SIZE > data.len() {
            return Err(RaError::Parse("mix 索引项截断".into()));
        }
        let id = read_i32(data, o);
        let offset = read_u32(data, o + 4);
        let size = read_u32(data, o + 8);
        entries.push(MixEntry { id, offset, size });
    }
    Ok(entries)
}

fn read_u16(data: &[u8], offset: usize) -> u16 {
    u16::from_le_bytes(data[offset..offset + 2].try_into().unwrap())
}

fn read_u32(data: &[u8], offset: usize) -> u32 {
    u32::from_le_bytes(data[offset..offset + 4].try_into().unwrap())
}

fn read_i32(data: &[u8], offset: usize) -> i32 {
    i32::from_le_bytes(data[offset..offset + 4].try_into().unwrap())
}
