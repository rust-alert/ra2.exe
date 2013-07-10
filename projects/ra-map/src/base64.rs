//! Base64 编解码（IsoMapPack5 等地图二进制段）。
//!
//! 标准字母表，跳过空白；地图文件把数据拆在编号 INI 键上。

/// 将 base64 字符串解码为原始字节。
///
/// 跳过空白与换行（地图把数据拆在编号 INI 行上）。遇非法字符返回错误。
pub fn base64_decode(input: &str) -> Result<Vec<u8>, String> {
    // Build a flat buffer of 6-bit values, skipping whitespace.
    let mut sextet_buf: Vec<u8> = Vec::with_capacity(input.len());
    let mut padding: usize = 0;

    for ch in input.chars() {
        if ch.is_ascii_whitespace() {
            continue;
        }
        if ch == '=' {
            padding += 1;
            continue;
        }
        if padding > 0 {
            return Err("Invalid base64: data after padding".into());
        }
        let val: u8 = decode_char(ch)?;
        sextet_buf.push(val);
    }

    if !(sextet_buf.len() + padding).is_multiple_of(4) {
        return Err(format!("Invalid base64: length {} (+ {} padding) is not a multiple of 4", sextet_buf.len(), padding));
    }

    // Each group of 4 sextets → 3 bytes. Final group may produce 1 or 2 bytes.
    let full_groups: usize = sextet_buf.len() / 4;
    let remainder: usize = sextet_buf.len() % 4;
    let mut output: Vec<u8> = Vec::with_capacity(full_groups * 3 + 2);

    for group in sextet_buf.chunks(4) {
        if group.len() == 4 {
            let combined: u32 = (group[0] as u32) << 18 | (group[1] as u32) << 12 | (group[2] as u32) << 6 | (group[3] as u32);
            output.push((combined >> 16) as u8);
            output.push((combined >> 8) as u8);
            output.push(combined as u8);
        }
        else if group.len() == 3 {
            // 3 sextets → 2 bytes (1 padding char).
            let combined: u32 = (group[0] as u32) << 18 | (group[1] as u32) << 12 | (group[2] as u32) << 6;
            output.push((combined >> 16) as u8);
            output.push((combined >> 8) as u8);
        }
        else if group.len() == 2 {
            // 2 sextets → 1 byte (2 padding chars).
            let combined: u32 = (group[0] as u32) << 18 | (group[1] as u32) << 12;
            output.push((combined >> 16) as u8);
        }
    }

    // If there was padding, trim the extra bytes from the last full group.
    if remainder == 0 && padding > 0 {
        let trim: usize = output.len().saturating_sub(padding);
        output.truncate(trim);
    }

    Ok(output)
}

/// 编码为标准 base64（含 `=` 填充），供往返校验与测试构造。
pub fn base64_encode(input: &[u8]) -> String {
    const TABLE: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut out = String::with_capacity((input.len() + 2) / 3 * 4);
    for chunk in input.chunks(3) {
        let b0 = chunk[0] as u32;
        let b1 = chunk.get(1).copied().unwrap_or(0) as u32;
        let b2 = chunk.get(2).copied().unwrap_or(0) as u32;
        let n = (b0 << 16) | (b1 << 8) | b2;
        out.push(TABLE[((n >> 18) & 0x3F) as usize] as char);
        out.push(TABLE[((n >> 12) & 0x3F) as usize] as char);
        if chunk.len() > 1 {
            out.push(TABLE[((n >> 6) & 0x3F) as usize] as char);
        }
        else {
            out.push('=');
        }
        if chunk.len() > 2 {
            out.push(TABLE[(n & 0x3F) as usize] as char);
        }
        else {
            out.push('=');
        }
    }
    out
}

/// Decode a single base64 character to its 6-bit value.
fn decode_char(ch: char) -> Result<u8, String> {
    match ch {
        'A'..='Z' => Ok(ch as u8 - b'A'),
        'a'..='z' => Ok(ch as u8 - b'a' + 26),
        '0'..='9' => Ok(ch as u8 - b'0' + 52),
        '+' => Ok(62),
        '/' => Ok(63),
        _ => Err(format!("Invalid base64 character: {:?}", ch)),
    }
}
