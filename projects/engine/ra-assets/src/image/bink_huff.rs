//! Bink 平面 Huffman 树描述：从码流读取符号置换 + 固定 VLC 编号。

use super::{
    bink_bits::{BitReader, VlcTable},
    bink_video::BinkVideoError,
};

/// 一棵 16 符号树：选用哪张固定 VLC，以及符号置换。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HuffmanTree {
    /// 固定树编号（0..=15）；0 表示恒等置换且码长全 4。
    pub vlc_num: u32,
    /// 解码得到的索引 → 业务符号。
    pub syms: [u8; 16],
}

impl Default for HuffmanTree {
    fn default() -> Self {
        let mut syms = [0u8; 16];
        for i in 0..16 {
            syms[i] = i as u8;
        }
        Self { vlc_num: 0, syms }
    }
}

impl HuffmanTree {
    /// 从码流读取树描述。
    pub fn read(r: &mut BitReader<'_>) -> Result<Self, BinkVideoError> {
        let vlc_num = r.read_bits(4)?;
        if vlc_num == 0 {
            return Ok(Self::default());
        }

        if r.read_bit()? {
            let mut len = r.read_bits(3)? as usize;
            let mut seen = [false; 16];
            let mut syms = [0u8; 16];
            for i in 0..=len {
                let s = r.read_bits(4)? as u8;
                syms[i] = s;
                seen[s as usize] = true;
            }
            let mut i = 0usize;
            while i < 16 && len < 15 {
                if !seen[i] {
                    len += 1;
                    syms[len] = i as u8;
                }
                i += 1;
            }
            Ok(Self { vlc_num, syms })
        }
        else {
            let len = r.read_bits(2)? as usize;
            let mut tmp1 = [0u8; 16];
            let mut tmp2 = [0u8; 16];
            let (mut in_arr, mut out_arr) = (&mut tmp1[..], &mut tmp2[..]);
            for i in 0..16 {
                in_arr[i] = i as u8;
            }
            for i in 0..=len {
                let size = 1usize << i;
                let mut t = 0usize;
                while t < 16 {
                    let mut src_window = [0u8; 16];
                    src_window[..2 * size].copy_from_slice(&in_arr[t..t + 2 * size]);
                    let mut dst_window = [0u8; 16];
                    merge_lists(r, &mut dst_window[..2 * size], &src_window[..2 * size], size)?;
                    out_arr[t..t + 2 * size].copy_from_slice(&dst_window[..2 * size]);
                    t += size << 1;
                }
                std::mem::swap(&mut in_arr, &mut out_arr);
            }
            let mut syms = [0u8; 16];
            syms.copy_from_slice(in_arr);
            Ok(Self { vlc_num, syms })
        }
    }

    /// 用固定 VLC 表解码一个符号。
    pub fn decode_sym(&self, tables: &[VlcTable; 16], r: &mut BitReader<'_>) -> Result<u8, BinkVideoError> {
        let idx = tables[self.vlc_num as usize].decode(r)? as usize;
        Ok(self.syms[idx])
    }
}

fn merge_lists(r: &mut BitReader<'_>, dst: &mut [u8], src: &[u8], size: usize) -> Result<(), BinkVideoError> {
    let mut src1 = 0usize;
    let mut src2 = size;
    let mut size1 = size;
    let mut size2 = size;
    let mut d = 0usize;
    while size1 > 0 && size2 > 0 {
        if !r.read_bit()? {
            dst[d] = src[src1];
            src1 += 1;
            size1 -= 1;
        }
        else {
            dst[d] = src[src2];
            src2 += 1;
            size2 -= 1;
        }
        d += 1;
    }
    while size1 > 0 {
        dst[d] = src[src1];
        src1 += 1;
        size1 -= 1;
        d += 1;
    }
    while size2 > 0 {
        dst[d] = src[src2];
        src2 += 1;
        size2 -= 1;
        d += 1;
    }
    Ok(())
}
