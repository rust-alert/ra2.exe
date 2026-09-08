//! LCW（Format80）解压：地图 `[OverlayPack]` / `[OverlayDataPack]`。
//!
//! 与 IsoMapPack5 的 LZO 不同。公开的 Westwood Format80 指令编码：
//! - `0x00..=0x7F`：相对回溯拷贝（2 字节指令）
//! - `0x80`：结束（低 6 位为 0 的字面量指令）
//! - `0x81..=0xBF`：字面量拷贝
//! - `0xC0..=0xFD`：短绝对拷贝
//! - `0xFE`：RLE 填充
//! - `0xFF`：长绝对拷贝
//!
//! 分块帧与 LZO 相同：`[u16 src_len][u16 dst_len][bytes…]`。

use std::fmt;

/// LCW 解压错误。
#[derive(Debug)]
pub enum LcwError {
    /// 输入字节不足。
    InputTruncated,
    /// 输出缓冲不够。
    OutputOverflow,
    /// 非法回溯引用。
    InvalidBackRef {
        /// 回溯源下标。
        src: usize,
        /// 当前写出位置。
        dest: usize,
    },
}

impl fmt::Display for LcwError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InputTruncated => write!(f, "LCW: 输入截断"),
            Self::OutputOverflow => write!(f, "LCW: 输出溢出"),
            Self::InvalidBackRef { src, dest } => {
                write!(f, "LCW: 非法回溯 src={src} dest={dest}")
            }
        }
    }
}

impl std::error::Error for LcwError {}

/// 将 LCW / Format80 解压到预分配缓冲，返回写入字节数。
pub fn lcw_decompress(src: &[u8], dest: &mut [u8]) -> Result<usize, LcwError> {
    let mut si = 0usize;
    let mut di = 0usize;

    loop {
        if si >= src.len() {
            return Ok(di);
        }
        let cmd = src[si];
        si += 1;

        if cmd & 0x80 == 0 {
            // 相对回溯：count = ((cmd & 0x70) >> 4) + 3，distance 为低 4 位拼下一字节。
            if si >= src.len() {
                return Err(LcwError::InputTruncated);
            }
            let second = src[si];
            si += 1;
            let count = usize::from((cmd & 0x70) >> 4) + 3;
            let distance = (usize::from(cmd & 0x0F) << 8) | usize::from(second);
            if distance > di {
                return Err(LcwError::InvalidBackRef { src: di.wrapping_sub(distance), dest: di });
            }
            if di + count > dest.len() {
                return Err(LcwError::OutputOverflow);
            }
            copy_overlap(dest, di, di - distance, count);
            di += count;
        }
        else if cmd & 0x40 == 0 {
            // 字面量；count=0 为结束标记。
            let count = usize::from(cmd & 0x3F);
            if count == 0 {
                return Ok(di);
            }
            if si + count > src.len() || di + count > dest.len() {
                return Err(if si + count > src.len() { LcwError::InputTruncated } else { LcwError::OutputOverflow });
            }
            dest[di..di + count].copy_from_slice(&src[si..si + count]);
            si += count;
            di += count;
        }
        else {
            let lower6 = cmd & 0x3F;
            if lower6 == 0x3E {
                // RLE：u16 次数 + 填充字节。
                if si + 3 > src.len() {
                    return Err(LcwError::InputTruncated);
                }
                let count = usize::from(u16::from_le_bytes([src[si], src[si + 1]]));
                let value = src[si + 2];
                si += 3;
                if di + count > dest.len() {
                    return Err(LcwError::OutputOverflow);
                }
                dest[di..di + count].fill(value);
                di += count;
            }
            else if lower6 == 0x3F {
                // 长绝对拷贝：u16 次数 + u16 源偏移。
                if si + 4 > src.len() {
                    return Err(LcwError::InputTruncated);
                }
                let count = usize::from(u16::from_le_bytes([src[si], src[si + 1]]));
                let abs_pos = usize::from(u16::from_le_bytes([src[si + 2], src[si + 3]]));
                si += 4;
                if abs_pos >= di {
                    return Err(LcwError::InvalidBackRef { src: abs_pos, dest: di });
                }
                if di + count > dest.len() {
                    return Err(LcwError::OutputOverflow);
                }
                copy_overlap(dest, di, abs_pos, count);
                di += count;
            }
            else {
                // 短绝对拷贝：count = lower6 + 3，后跟 u16 源偏移。
                let count = usize::from(lower6) + 3;
                if si + 2 > src.len() {
                    return Err(LcwError::InputTruncated);
                }
                let abs_pos = usize::from(u16::from_le_bytes([src[si], src[si + 1]]));
                si += 2;
                if abs_pos >= di {
                    return Err(LcwError::InvalidBackRef { src: abs_pos, dest: di });
                }
                if di + count > dest.len() {
                    return Err(LcwError::OutputOverflow);
                }
                copy_overlap(dest, di, abs_pos, count);
                di += count;
            }
        }
    }
}

/// 分块 LCW：与 IsoMapPack 相同的 `[src_len][dst_len][payload]` 帧。
pub fn decompress_chunks(data: &[u8]) -> Result<Vec<u8>, LcwError> {
    let mut output = Vec::new();
    let mut pos = 0usize;

    while pos + 4 <= data.len() {
        let src_len = usize::from(u16::from_le_bytes([data[pos], data[pos + 1]]));
        let dst_len = usize::from(u16::from_le_bytes([data[pos + 2], data[pos + 3]]));
        pos += 4;

        if src_len == 0 && dst_len == 0 {
            break;
        }
        if pos + src_len > data.len() {
            return Err(LcwError::InputTruncated);
        }

        let mut chunk = vec![0u8; dst_len.max(1)];
        let written = lcw_decompress(&data[pos..pos + src_len], &mut chunk)?;
        let copy_len = dst_len.min(written);
        output.extend_from_slice(&chunk[..copy_len]);
        pos += src_len;
    }

    Ok(output)
}

fn copy_overlap(dest: &mut [u8], di: usize, src: usize, count: usize) {
    if di > src && di - src == 1 {
        let val = dest[di - 1];
        dest[di..di + count].fill(val);
    }
    else {
        for i in 0..count {
            dest[di + i] = dest[src + i];
        }
    }
}
