//! LZO1X 解压：IsoMapPack5 地形块。
//!
//! 块格式：[u16 src_len][u16 dst_len][compressed_bytes] 重复至耗尽。

use std::fmt;

/// LZO 解压错误。
#[derive(Debug)]
pub enum LzoError {
    /// 输入字节不足。
    InputTruncated,
    /// 输出缓冲不够。
    OutputOverflow,
    /// 非法回溯引用。
    InvalidBackRef {
        /// 回溯距离。
        distance: usize,
        /// 当前写出位置。
        output_pos: usize,
    },
}

impl fmt::Display for LzoError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            LzoError::InputTruncated => write!(f, "LZO: 输入截断"),
            LzoError::OutputOverflow => write!(f, "LZO: 输出溢出"),
            LzoError::InvalidBackRef { distance, output_pos } => {
                write!(f, "LZO: 非法回溯 distance={distance} output_pos={output_pos}")
            }
        }
    }
}

impl std::error::Error for LzoError {}

/// 将 LZO1X 压缩数据解压到 `dst`，返回写入字节数。
pub fn lzo1x_decompress(src: &[u8], dst: &mut [u8]) -> Result<usize, LzoError> {
    let mut ip: usize = 0; // input position
    let mut op: usize = 0; // output position

    if src.is_empty() {
        return Ok(0);
    }

    // --- First byte special handling ---
    let mut cmd: u8 = read_byte(src, &mut ip)?;
    let mut state: usize;

    if cmd > 17 {
        // Literal run of (cmd - 17) bytes.
        let count: usize = (cmd - 17) as usize;
        copy_literals(src, &mut ip, dst, &mut op, count)?;
        if count < 4 {
            state = count;
            cmd = read_byte(src, &mut ip)?;
            // Fall into match handling below.
        }
        else {
            // >= 4 literals copied, read next cmd for first_literal_run path.
            state = 4;
            cmd = read_byte(src, &mut ip)?;
            if cmd < 16 {
                // M1' match: distance with +2048 offset.
                let dist: usize = ((cmd >> 2) as usize) + ((read_byte(src, &mut ip)? as usize) << 2) + 2049;
                copy_match(dst, op, dist, 3)?;
                op += 3;
                state = (cmd & 3) as usize;
                if state > 0 {
                    copy_literals(src, &mut ip, dst, &mut op, state)?;
                }
                cmd = read_byte(src, &mut ip)?;
                // Now in the main match loop with state set.
            }
            // If cmd >= 16, fall through to match handling.
        }
    }
    else {
        state = 0;
        // cmd is the first instruction, fall into main loop.
    }

    // --- Main loop ---
    loop {
        if cmd < 16 {
            if state == 0 {
                // Long literal run.
                let mut length: usize = cmd as usize;
                if length == 0 {
                    length = 15;
                    length += read_vle(src, &mut ip)?;
                }
                length += 3;
                copy_literals(src, &mut ip, dst, &mut op, length)?;
                // First-literal-run path: next cmd may be M1' match.
                cmd = read_byte(src, &mut ip)?;
                if cmd < 16 {
                    let dist: usize = ((cmd >> 2) as usize) + ((read_byte(src, &mut ip)? as usize) << 2) + 2049;
                    copy_match(dst, op, dist, 3)?;
                    op += 3;
                    state = (cmd & 3) as usize;
                    if state > 0 {
                        copy_literals(src, &mut ip, dst, &mut op, state)?;
                    }
                    cmd = read_byte(src, &mut ip)?;
                    continue;
                }
                // cmd >= 16, fall through to match handling.
            }
            else {
                // M1 short match (state >= 1): copy 2 bytes.
                let dist: usize = ((cmd >> 2) as usize) + ((read_byte(src, &mut ip)? as usize) << 2) + 1;
                copy_match(dst, op, dist, 2)?;
                op += 2;
                state = (cmd & 3) as usize;
                if state > 0 {
                    copy_literals(src, &mut ip, dst, &mut op, state)?;
                }
                cmd = read_byte(src, &mut ip)?;
                continue;
            }
        }

        // --- Match instructions (cmd >= 16) ---
        let (lzo_length, dist): (usize, usize);

        if cmd >= 64 {
            // M2: short match, 1..2048 distance, 3..8 length.
            let len_part: usize = (cmd >> 5) as usize; // 2..7
            let dist_lo: usize = ((cmd >> 2) & 7) as usize;
            let dist_hi: usize = read_byte(src, &mut ip)? as usize;
            dist = (dist_hi << 3) + dist_lo + 1;
            lzo_length = len_part + 1; // 3..8
        }
        else if cmd >= 32 {
            // M3: medium match, 1..16384 distance.
            let mut length: usize = (cmd & 31) as usize;
            if length == 0 {
                length = 31;
                length += read_vle(src, &mut ip)?;
            }
            lzo_length = length + 2;
            let word: u16 = read_u16_le_lzo(src, &mut ip)?;
            dist = ((word >> 2) as usize) + 1;
            state = (word & 3) as usize;
            copy_match(dst, op, dist, lzo_length)?;
            op += lzo_length;
            if state > 0 {
                copy_literals(src, &mut ip, dst, &mut op, state)?;
            }
            cmd = read_byte(src, &mut ip)?;
            continue;
        }
        else {
            // M4: long match (16..31), or end-of-stream.
            let mut length: usize = (cmd & 7) as usize;
            if length == 0 {
                length = 7;
                length += read_vle(src, &mut ip)?;
            }
            lzo_length = length + 2;
            let high_dist: usize = ((cmd as usize) & 8) << 11;
            let word: u16 = read_u16_le_lzo(src, &mut ip)?;
            let low_dist: usize = (word >> 2) as usize;

            // End-of-stream: high_dist + low_dist == 0.
            if high_dist == 0 && low_dist == 0 {
                return Ok(op);
            }
            dist = high_dist + low_dist + 16384;
            state = (word & 3) as usize;
            copy_match(dst, op, dist, lzo_length)?;
            op += lzo_length;
            if state > 0 {
                copy_literals(src, &mut ip, dst, &mut op, state)?;
            }
            cmd = read_byte(src, &mut ip)?;
            continue;
        }

        // M2 match — state (literal count after match) is in the command byte's
        // low 2 bits, NOT the distance byte. M2 format: [LLL][DDD][SS] where
        // SS = state, DDD = dist_lo, LLL = length.
        state = (cmd & 3) as usize;
        copy_match(dst, op, dist, lzo_length)?;
        op += lzo_length;
        if state > 0 {
            copy_literals(src, &mut ip, dst, &mut op, state)?;
        }
        cmd = read_byte(src, &mut ip)?;
    }
}

/// 解压 IsoMapPack5 分块帧 LZO 数据。
///
/// 块格式：重复的 `[u16 src_len][u16 dst_len][compressed_bytes]`，读完输入为止。
pub fn decompress_chunks(data: &[u8]) -> Result<Vec<u8>, LzoError> {
    let mut output: Vec<u8> = Vec::new();
    let mut pos: usize = 0;

    while pos + 4 <= data.len() {
        let src_len: usize = u16::from_le_bytes([data[pos], data[pos + 1]]) as usize;
        let dst_len: usize = u16::from_le_bytes([data[pos + 2], data[pos + 3]]) as usize;
        pos += 4;

        if src_len == 0 && dst_len == 0 {
            break;
        }
        if pos + src_len > data.len() {
            return Err(LzoError::InputTruncated);
        }

        let mut chunk_out: Vec<u8> = vec![0u8; dst_len];
        let written: usize = lzo1x_decompress(&data[pos..pos + src_len], &mut chunk_out)?;
        chunk_out.truncate(written);
        output.extend_from_slice(&chunk_out);
        pos += src_len;
    }

    Ok(output)
}

// --- Internal helpers ---

/// Read a single byte, advancing the position.
fn read_byte(src: &[u8], pos: &mut usize) -> Result<u8, LzoError> {
    if *pos >= src.len() {
        return Err(LzoError::InputTruncated);
    }
    let b: u8 = src[*pos];
    *pos += 1;
    Ok(b)
}

/// Read a u16 LE value, advancing the position by 2.
fn read_u16_le_lzo(src: &[u8], pos: &mut usize) -> Result<u16, LzoError> {
    if *pos + 2 > src.len() {
        return Err(LzoError::InputTruncated);
    }
    let val: u16 = u16::from_le_bytes([src[*pos], src[*pos + 1]]);
    *pos += 2;
    Ok(val)
}

/// Read variable-length extension: skip zero bytes (each adds 255) then add the
/// first non-zero byte.
fn read_vle(src: &[u8], pos: &mut usize) -> Result<usize, LzoError> {
    let mut extra: usize = 0;
    loop {
        let b: u8 = read_byte(src, pos)?;
        if b != 0 {
            return Ok(extra + b as usize);
        }
        extra += 255;
    }
}

/// Copy `count` literal bytes from input to output.
fn copy_literals(src: &[u8], ip: &mut usize, dst: &mut [u8], op: &mut usize, count: usize) -> Result<(), LzoError> {
    if *ip + count > src.len() {
        return Err(LzoError::InputTruncated);
    }
    if *op + count > dst.len() {
        return Err(LzoError::OutputOverflow);
    }
    dst[*op..*op + count].copy_from_slice(&src[*ip..*ip + count]);
    *ip += count;
    *op += count;
    Ok(())
}

/// Copy `count` bytes from a back-reference in the output buffer.
/// Must copy byte-by-byte because source and destination may overlap.
fn copy_match(dst: &mut [u8], op: usize, distance: usize, count: usize) -> Result<(), LzoError> {
    if distance > op {
        return Err(LzoError::InvalidBackRef { distance, output_pos: op });
    }
    if op + count > dst.len() {
        return Err(LzoError::OutputOverflow);
    }
    let mut src_pos: usize = op - distance;
    let mut dst_pos: usize = op;
    for _ in 0..count {
        dst[dst_pos] = dst[src_pos];
        src_pos += 1;
        dst_pos += 1;
    }
    Ok(())
}
