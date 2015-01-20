//! CSF 本地化字符串表（如 `ra2.csf` / `ra2md.csf`）。
//!
//! 头：` FSC` + version / label_count / string_count / reserved / language。
//! 标签：` LBL` + 对数 + ASCII 名 + 一条或多条 ` RTS`/`WRTS` 值（UTF-16-LE 按位取反）。

use std::collections::HashMap;

use ra_types::{RaError, RaResult};

const HEADER_MAGIC: u32 = 0x4353_4620; // " FSC"
/// 标签块魔数（` LBL`）。
pub const LABEL_MAGIC: u32 = 0x4C42_4C20;
/// 字符串块魔数（` RTS`）。
pub const STRING_MAGIC: u32 = 0x5354_5220;
const STRING_EXTRA_MAGIC: u32 = 0x5354_5257; // "WRTS"
const MIN_FILE_SIZE: usize = 24;

/// 已解析的 CSF 表（键为大写，查找大小写不敏感）。
#[derive(Debug, Clone)]
pub struct CsfFile {
    /// 文件版本。
    pub version: u32,
    /// 语言 id（version≥2 时有效）。
    pub language: u32,
    entries: HashMap<String, String>,
}

impl CsfFile {
    /// 从原始字节解析。
    pub fn parse(data: &[u8]) -> RaResult<Self> {
        if data.len() < MIN_FILE_SIZE {
            return Err(RaError::Msg(format!("CSF 过短 · {} < {MIN_FILE_SIZE}", data.len())));
        }
        let magic = read_u32(data, 0)?;
        if magic != HEADER_MAGIC {
            return Err(RaError::Msg(format!("CSF 魔数错误 · 0x{magic:08X}")));
        }
        let version = read_u32(data, 4)?;
        let label_count = read_u32(data, 8)?;
        let _string_count = read_u32(data, 12)?;
        let language = if version >= 2 { read_u32(data, 20)? } else { 0 };

        let mut entries = HashMap::new();
        let mut offset = MIN_FILE_SIZE;
        for _ in 0..label_count {
            let (label, value, next) = read_label_entry(data, offset)?;
            entries.insert(label, value);
            offset = next;
        }

        Ok(Self { version, language, entries })
    }

    /// 按标签名取值（大小写不敏感）。
    pub fn get(&self, label: &str) -> Option<&str> {
        self.entries.get(&label.to_ascii_uppercase()).map(String::as_str)
    }

    /// 条目数。
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// 是否为空。
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// 遍历全部条目（键已为大写）。
    pub fn iter(&self) -> impl Iterator<Item = (&str, &str)> + '_ {
        self.entries.iter().map(|(k, v)| (k.as_str(), v.as_str()))
    }

    /// 导出为 UTF-8 文本表：每行 `KEY=value`，值内换行写成 `\n`，按键排序。
    pub fn to_text_table(&self) -> String {
        let mut keys: Vec<&str> = self.entries.keys().map(String::as_str).collect();
        keys.sort_unstable();
        let mut out = String::new();
        for key in keys {
            let value = self.entries.get(key).map(String::as_str).unwrap_or("");
            out.push_str(key);
            out.push('=');
            for ch in value.chars() {
                match ch {
                    '\n' => out.push_str("\\n"),
                    '\r' => out.push_str("\\r"),
                    '\\' => out.push_str("\\\\"),
                    c => out.push(c),
                }
            }
            out.push('\n');
        }
        out
    }
}

fn read_u32(data: &[u8], offset: usize) -> RaResult<u32> {
    let end = offset.checked_add(4).ok_or_else(|| RaError::Msg("CSF 偏移溢出".into()))?;
    if end > data.len() {
        return Err(RaError::Msg("CSF 截断读 u32".into()));
    }
    Ok(u32::from_le_bytes(data[offset..end].try_into().unwrap()))
}

fn read_label_entry(data: &[u8], mut offset: usize) -> RaResult<(String, String, usize)> {
    let magic = read_u32(data, offset)?;
    if magic != LABEL_MAGIC {
        return Err(RaError::Msg(format!("CSF 标签魔数错误 · 0x{magic:08X} @ {offset}")));
    }
    offset += 4;
    let pair_count = read_u32(data, offset)? as usize;
    offset += 4;
    let name_len = read_u32(data, offset)? as usize;
    offset += 4;
    let name_end = offset.checked_add(name_len).ok_or_else(|| RaError::Msg("CSF 标签名溢出".into()))?;
    if name_end > data.len() {
        return Err(RaError::Msg("CSF 标签名截断".into()));
    }
    let name = String::from_utf8_lossy(&data[offset..name_end]).to_ascii_uppercase();
    offset = name_end;

    let mut value = String::new();
    for i in 0..pair_count {
        let (s, next) = read_string_value(data, offset)?;
        offset = next;
        if i == 0 {
            value = s;
        }
    }
    // pair_count 为 0 时保留空串。
    Ok((name, value, offset))
}

fn read_string_value(data: &[u8], mut offset: usize) -> RaResult<(String, usize)> {
    let magic = read_u32(data, offset)?;
    offset += 4;
    let has_extra = magic == STRING_EXTRA_MAGIC;
    if magic != STRING_MAGIC && !has_extra {
        return Err(RaError::Msg(format!("CSF 字符串魔数错误 · 0x{magic:08X}")));
    }
    let char_count = read_u32(data, offset)? as usize;
    offset += 4;
    let byte_len = char_count.saturating_mul(2);
    let end = offset.checked_add(byte_len).ok_or_else(|| RaError::Msg("CSF 字符串溢出".into()))?;
    if end > data.len() {
        return Err(RaError::Msg("CSF 字符串截断".into()));
    }
    let mut units = Vec::with_capacity(char_count);
    for i in 0..char_count {
        let o = offset + i * 2;
        let raw = u16::from_le_bytes([data[o], data[o + 1]]);
        units.push(!raw);
    }
    offset = end;
    if has_extra {
        let extra_len = read_u32(data, offset)? as usize;
        offset += 4;
        offset = offset.checked_add(extra_len).ok_or_else(|| RaError::Msg("CSF 附加数据溢出".into()))?;
        if offset > data.len() {
            return Err(RaError::Msg("CSF 附加数据截断".into()));
        }
    }
    Ok((String::from_utf16_lossy(&units), offset))
}
