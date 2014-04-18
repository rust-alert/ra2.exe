//! Bink 平面解码用的 9 路数据捆（bundle）：树描述 + 值缓冲。

use super::bink::BinkVersion;
use super::bink_bits::{BitReader, VlcTable};
use super::bink_huff::HuffmanTree;
use super::bink_tables::BINK_RLELENS;
use super::bink_video::BinkVideoError;

/// Bundle 种类数。
pub const NB_SRC: usize = 9;

/// 一帧平面内各值源。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(usize)]
pub enum BinkSrc {
    /// 8×8 块类型。
    BlockTypes = 0,
    /// 缩放子块类型。
    SubBlockTypes = 1,
    /// 颜色字节。
    Colors = 2,
    /// 图案选择。
    Pattern = 3,
    /// 运动 X。
    XOff = 4,
    /// 运动 Y。
    YOff = 5,
    /// 帧内 DC。
    IntraDc = 6,
    /// 帧间 DC。
    InterDc = 7,
    /// RLE 游程。
    Run = 8,
}

/// 单个 bundle 的游标与树。
#[derive(Debug, Clone)]
pub struct BinkBundle {
    /// 本行 refill 长度字段位数。
    pub len_bits: u32,
    /// Huffman 树。
    pub tree: HuffmanTree,
    /// 在共享缓冲中的起点。
    pub buf_start: usize,
    /// 终点（不含）。
    pub buf_end: usize,
    /// 写入游标。
    pub cur_dec: usize,
    /// 读取游标。
    pub cur_ptr: usize,
    /// 本行不再 refill。
    pub skip_fills: bool,
}

impl BinkBundle {
    /// 按块容量切出一段共享缓冲。
    pub fn slice(blocks: usize, index: usize) -> Self {
        let per = blocks.saturating_mul(64);
        let start = index.saturating_mul(per);
        Self {
            len_bits: 0,
            tree: HuffmanTree::default(),
            buf_start: start,
            buf_end: start + per,
            cur_dec: start,
            cur_ptr: start,
            skip_fills: false,
        }
    }
}

/// `floor(log2(x))`，`x == 0` 时为 0。
pub fn log2_floor(x: u32) -> u32 {
    if x == 0 {
        0
    } else {
        31 - x.leading_zeros()
    }
}

/// 按平面宽与块列数设置各 bundle 的 `len_bits`。
pub fn init_bundle_lengths(bundles: &mut [BinkBundle; NB_SRC], width: u32, bw: u32) {
    let width = (width + 7) & !7;
    let log2_bw_plus_511 = log2_floor((width >> 3) + 511) + 1;
    let log2_bw16_plus_511 = log2_floor((width >> 4) + 511) + 1;
    let log2_bw_cols_plus_511 = log2_floor(bw * 64 + 511) + 1;
    let log2_pattern_plus_511 = log2_floor((bw << 3) + 511) + 1;
    let log2_run_plus_511 = log2_floor(bw * 48 + 511) + 1;

    bundles[BinkSrc::BlockTypes as usize].len_bits = log2_bw_plus_511;
    bundles[BinkSrc::SubBlockTypes as usize].len_bits = log2_bw16_plus_511;
    bundles[BinkSrc::Colors as usize].len_bits = log2_bw_cols_plus_511;
    bundles[BinkSrc::IntraDc as usize].len_bits = log2_bw_plus_511;
    bundles[BinkSrc::InterDc as usize].len_bits = log2_bw_plus_511;
    bundles[BinkSrc::XOff as usize].len_bits = log2_bw_plus_511;
    bundles[BinkSrc::YOff as usize].len_bits = log2_bw_plus_511;
    bundles[BinkSrc::Pattern as usize].len_bits = log2_pattern_plus_511;
    bundles[BinkSrc::Run as usize].len_bits = log2_run_plus_511;
}

/// 为某一 bundle 读取树并重置游标。
pub fn read_bundle(
    r: &mut BitReader<'_>,
    bundles: &mut [BinkBundle; NB_SRC],
    col_high: &mut [HuffmanTree; 16],
    col_lastval: &mut u8,
    bundle_num: usize,
) -> Result<(), BinkVideoError> {
    if bundle_num == BinkSrc::Colors as usize {
        for i in 0..16 {
            col_high[i] = HuffmanTree::read(r)?;
        }
        *col_lastval = 0;
    }
    if bundle_num != BinkSrc::IntraDc as usize && bundle_num != BinkSrc::InterDc as usize {
        bundles[bundle_num].tree = HuffmanTree::read(r)?;
    }
    let b = &mut bundles[bundle_num];
    b.cur_dec = b.buf_start;
    b.cur_ptr = b.buf_start;
    b.skip_fills = false;
    Ok(())
}

/// 读出下一个已解码的 bundle 值。
pub fn take_value(bundles: &mut [BinkBundle; NB_SRC], data: &[u8], bundle_num: usize) -> u8 {
    let b = &mut bundles[bundle_num];
    let v = data.get(b.cur_ptr).copied().unwrap_or(0);
    b.cur_ptr = b.cur_ptr.saturating_add(1);
    v
}

/// 填充块类型（或子块类型）bundle 一行的值。
pub fn read_block_types(
    r: &mut BitReader<'_>,
    bundles: &mut [BinkBundle; NB_SRC],
    data: &mut [u8],
    vlc: &[VlcTable; 16],
    version: BinkVersion,
    bundle_num: usize,
) -> Result<(), BinkVideoError> {
    let (len_bits, buf_end, tree, cur_dec_start) = {
        let b = &bundles[bundle_num];
        if b.skip_fills || b.cur_dec > b.cur_ptr {
            return Ok(());
        }
        (b.len_bits, b.buf_end, b.tree.clone(), b.cur_dec)
    };
    let t_raw = r.read_bits(len_bits)?;
    if t_raw == 0 {
        bundles[bundle_num].skip_fills = true;
        return Ok(());
    }
    let t = if version == BinkVersion::BikK {
        let xored = t_raw ^ 0xBB;
        if xored == 0 {
            bundles[bundle_num].skip_fills = true;
            return Ok(());
        }
        xored
    } else {
        t_raw
    } as usize;
    let dec_end = cur_dec_start.saturating_add(t);
    if dec_end > buf_end {
        return Err(BinkVideoError::Msg("块类型值过多".into()));
    }
    if r.read_bit()? {
        let v = r.read_bits(4)? as u8;
        data[cur_dec_start..dec_end].fill(v);
    } else {
        let mut last: u8 = 0;
        let mut dec = cur_dec_start;
        while dec < dec_end {
            let v = tree.decode_sym(vlc, r)?;
            if v < 12 {
                last = v;
                data[dec] = v;
                dec += 1;
            } else {
                let run = BINK_RLELENS[(v - 12) as usize] as usize;
                if dec_end.saturating_sub(dec) < run {
                    return Err(BinkVideoError::Msg("块类型 RLE 越界".into()));
                }
                data[dec..dec + run].fill(last);
                dec += run;
            }
        }
    }
    bundles[bundle_num].cur_dec = dec_end;
    Ok(())
}

/// 填充颜色 bundle：高半字节走 `col_high[col_lastval]`，低半字节走本树。
pub fn read_colors(
    r: &mut BitReader<'_>,
    bundles: &mut [BinkBundle; NB_SRC],
    data: &mut [u8],
    vlc: &[VlcTable; 16],
    col_high: &[HuffmanTree; 16],
    col_lastval: &mut u8,
    bundle_num: usize,
) -> Result<(), BinkVideoError> {
    let (len_bits, buf_end, tree, cur_dec_start) = {
        let b = &bundles[bundle_num];
        if b.skip_fills || b.cur_dec > b.cur_ptr {
            return Ok(());
        }
        (b.len_bits, b.buf_end, b.tree.clone(), b.cur_dec)
    };
    let t = r.read_bits(len_bits)? as usize;
    if t == 0 {
        bundles[bundle_num].skip_fills = true;
        return Ok(());
    }
    let dec_end = cur_dec_start.saturating_add(t);
    if dec_end > buf_end {
        return Err(BinkVideoError::Msg("颜色值过多".into()));
    }
    if r.read_bit()? {
        let hi = col_high[(*col_lastval & 0xF) as usize].decode_sym(vlc, r)?;
        *col_lastval = hi;
        let lo = tree.decode_sym(vlc, r)?;
        let v = (hi << 4) | lo;
        data[cur_dec_start..dec_end].fill(v);
    } else {
        let mut dec = cur_dec_start;
        while dec < dec_end {
            let hi = col_high[(*col_lastval & 0xF) as usize].decode_sym(vlc, r)?;
            *col_lastval = hi;
            let lo = tree.decode_sym(vlc, r)?;
            data[dec] = (hi << 4) | lo;
            dec += 1;
        }
    }
    bundles[bundle_num].cur_dec = dec_end;
    Ok(())
}

/// 填充图案 bundle：每字节由两个 4 位 Huffman 符号拼成。
pub fn read_patterns(
    r: &mut BitReader<'_>,
    bundles: &mut [BinkBundle; NB_SRC],
    data: &mut [u8],
    vlc: &[VlcTable; 16],
    bundle_num: usize,
) -> Result<(), BinkVideoError> {
    let (len_bits, buf_end, tree, cur_dec_start) = {
        let b = &bundles[bundle_num];
        if b.skip_fills || b.cur_dec > b.cur_ptr {
            return Ok(());
        }
        (b.len_bits, b.buf_end, b.tree.clone(), b.cur_dec)
    };
    let t = r.read_bits(len_bits)? as usize;
    if t == 0 {
        bundles[bundle_num].skip_fills = true;
        return Ok(());
    }
    let dec_end = cur_dec_start.saturating_add(t);
    if dec_end > buf_end {
        return Err(BinkVideoError::Msg("图案值过多".into()));
    }
    let mut dec = cur_dec_start;
    while dec < dec_end {
        let lo = tree.decode_sym(vlc, r)?;
        let hi = tree.decode_sym(vlc, r)?;
        data[dec] = lo | (hi << 4);
        dec += 1;
    }
    bundles[bundle_num].cur_dec = dec_end;
    Ok(())
}

/// 填充运动偏移 bundle（有符号 i8 存成 `u8` 位型）。
pub fn read_motion_values(
    r: &mut BitReader<'_>,
    bundles: &mut [BinkBundle; NB_SRC],
    data: &mut [u8],
    vlc: &[VlcTable; 16],
    bundle_num: usize,
) -> Result<(), BinkVideoError> {
    let (len_bits, buf_end, tree, cur_dec_start) = {
        let b = &bundles[bundle_num];
        if b.skip_fills || b.cur_dec > b.cur_ptr {
            return Ok(());
        }
        (b.len_bits, b.buf_end, b.tree.clone(), b.cur_dec)
    };
    let t = r.read_bits(len_bits)? as usize;
    if t == 0 {
        bundles[bundle_num].skip_fills = true;
        return Ok(());
    }
    let dec_end = cur_dec_start.saturating_add(t);
    if dec_end > buf_end {
        return Err(BinkVideoError::Msg("运动偏移过多".into()));
    }
    if r.read_bit()? {
        let mut v = r.read_bits(4)? as i32;
        if v != 0 {
            let sign = if r.read_bit()? { -1 } else { 0 };
            v = (v ^ sign) - sign;
        }
        data[cur_dec_start..dec_end].fill(v as i8 as u8);
    } else {
        let mut dec = cur_dec_start;
        while dec < dec_end {
            let mut v = tree.decode_sym(vlc, r)? as i32;
            if v != 0 {
                let sign = if r.read_bit()? { -1 } else { 0 };
                v = (v ^ sign) - sign;
            }
            data[dec] = v as i8 as u8;
            dec += 1;
        }
    }
    bundles[bundle_num].cur_dec = dec_end;
    Ok(())
}

/// 填充 RLE 游程 bundle（无符号）。
pub fn read_runs(
    r: &mut BitReader<'_>,
    bundles: &mut [BinkBundle; NB_SRC],
    data: &mut [u8],
    vlc: &[VlcTable; 16],
    bundle_num: usize,
) -> Result<(), BinkVideoError> {
    let (len_bits, buf_end, tree, cur_dec_start) = {
        let b = &bundles[bundle_num];
        if b.skip_fills || b.cur_dec > b.cur_ptr {
            return Ok(());
        }
        (b.len_bits, b.buf_end, b.tree.clone(), b.cur_dec)
    };
    let t = r.read_bits(len_bits)? as usize;
    if t == 0 {
        bundles[bundle_num].skip_fills = true;
        return Ok(());
    }
    let dec_end = cur_dec_start.saturating_add(t);
    if dec_end > buf_end {
        return Err(BinkVideoError::Msg("游程值过多".into()));
    }
    if r.read_bit()? {
        let v = r.read_bits(4)? as u8;
        data[cur_dec_start..dec_end].fill(v);
    } else {
        let mut dec = cur_dec_start;
        while dec < dec_end {
            data[dec] = tree.decode_sym(vlc, r)?;
            dec += 1;
        }
    }
    bundles[bundle_num].cur_dec = dec_end;
    Ok(())
}

fn write_i16(data: &mut [u8], offset: usize, v: i16) {
    let bytes = v.to_le_bytes();
    if offset + 1 < data.len() {
        data[offset] = bytes[0];
        data[offset + 1] = bytes[1];
    }
}

fn read_i16(data: &[u8], offset: usize) -> i16 {
    let lo = data.get(offset).copied().unwrap_or(0);
    let hi = data.get(offset + 1).copied().unwrap_or(0);
    i16::from_le_bytes([lo, hi])
}

/// 读出下一个已解码的 16 位 DC 值（小端）。
pub fn take_value16(bundles: &mut [BinkBundle; NB_SRC], data: &[u8], bundle_num: usize) -> i16 {
    let b = &mut bundles[bundle_num];
    let v = read_i16(data, b.cur_ptr);
    b.cur_ptr = b.cur_ptr.saturating_add(2);
    v
}

/// 填充帧内 / 帧间 DC bundle（小端 i16，增量编码）。
pub fn read_dcs(
    r: &mut BitReader<'_>,
    bundles: &mut [BinkBundle; NB_SRC],
    data: &mut [u8],
    bundle_num: usize,
    start_bits: u32,
    has_sign: bool,
) -> Result<(), BinkVideoError> {
    let (len_bits, buf_end, cur_dec_start) = {
        let b = &bundles[bundle_num];
        if b.skip_fills || b.cur_dec > b.cur_ptr {
            return Ok(());
        }
        (b.len_bits, b.buf_end, b.cur_dec)
    };
    let mut len = r.read_bits(len_bits)? as i32;
    if len == 0 {
        bundles[bundle_num].skip_fills = true;
        return Ok(());
    }
    let first_bits = start_bits - if has_sign { 1 } else { 0 };
    let mut v = r.read_bits(first_bits)? as i32;
    if v != 0 && has_sign {
        let sign = if r.read_bit()? { -1 } else { 0 };
        v = (v ^ sign) - sign;
    }
    let remaining_i16s = (buf_end - cur_dec_start) / 2;
    if remaining_i16s < 1 {
        return Err(BinkVideoError::Msg("DC 缓冲已满".into()));
    }
    let mut dec = cur_dec_start;
    write_i16(data, dec, v as i16);
    dec += 2;
    len -= 1;

    let mut i = 0i32;
    while i < len {
        let len2 = (len - i).min(8);
        let remaining = ((buf_end - dec) / 2) as i32;
        if remaining < len2 {
            return Err(BinkVideoError::Msg("DC 游程越界".into()));
        }
        let bsize = r.read_bits(4)?;
        if bsize != 0 {
            for _ in 0..len2 {
                let mut v2 = r.read_bits(bsize)? as i32;
                if v2 != 0 {
                    let sign = if r.read_bit()? { -1 } else { 0 };
                    v2 = (v2 ^ sign) - sign;
                }
                v += v2;
                if !(-32768..=32767).contains(&v) {
                    return Err(BinkVideoError::Msg(format!("DC 越界：{v}")));
                }
                write_i16(data, dec, v as i16);
                dec += 2;
            }
        } else {
            for _ in 0..len2 {
                write_i16(data, dec, v as i16);
                dec += 2;
            }
        }
        i += 8;
    }
    bundles[bundle_num].cur_dec = dec;
    Ok(())
}

/// 分配 9 路 bundle 与共享缓冲。
pub fn alloc_bundles(width: u32, height: u32) -> ([BinkBundle; NB_SRC], Vec<u8>) {
    let bw = ((width + 7) >> 3) as usize;
    let bh = ((height + 7) >> 3) as usize;
    let blocks = bw.saturating_mul(bh);
    let total = blocks.saturating_mul(64 * NB_SRC);
    let data = vec![0u8; total];
    let bundles = std::array::from_fn(|i| BinkBundle::slice(blocks, i));
    (bundles, data)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn init_lengths_for_320_wide() {
        let (mut bundles, _) = alloc_bundles(320, 240);
        let bw = (320 + 7) >> 3;
        init_bundle_lengths(&mut bundles, 320, bw);
        assert!(bundles[BinkSrc::BlockTypes as usize].len_bits >= 1);
        assert_eq!(
            bundles[BinkSrc::BlockTypes as usize].len_bits,
            bundles[BinkSrc::XOff as usize].len_bits
        );
    }

    #[test]
    fn read_bundle_identity_tree() {
        let (mut bundles, _) = alloc_bundles(16, 16);
        init_bundle_lengths(&mut bundles, 16, 2);
        // vlc_num = 0
        let data = [0x00u8];
        let mut r = BitReader::from_bytes(&data);
        let mut col_high = std::array::from_fn(|_| HuffmanTree::default());
        let mut col_last = 0u8;
        read_bundle(
            &mut r,
            &mut bundles,
            &mut col_high,
            &mut col_last,
            BinkSrc::BlockTypes as usize,
        )
        .unwrap();
        assert_eq!(bundles[0].tree.vlc_num, 0);
        assert!(!bundles[0].skip_fills);
    }
}
