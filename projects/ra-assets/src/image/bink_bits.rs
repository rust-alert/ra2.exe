//! Bink 码流按位读取（字节内 LSB 优先，跨字节低位先出）。

use super::bink_video::BinkVideoError;

/// 码流阅读器。
#[derive(Debug, Clone)]
pub struct BitReader<'a> {
    data: &'a [u8],
    bit_pos: usize,
    bits_total: usize,
}

impl<'a> BitReader<'a> {
    /// 在 `data` 上限制可读位数（不得超过 `data.len() * 8`）。
    pub fn new(data: &'a [u8], bits_total: usize) -> Self {
        let max = data.len().saturating_mul(8);
        Self {
            data,
            bit_pos: 0,
            bits_total: bits_total.min(max),
        }
    }

    /// 覆盖整段字节。
    pub fn from_bytes(data: &'a [u8]) -> Self {
        Self::new(data, data.len().saturating_mul(8))
    }

    /// 已读位数。
    pub fn pos(&self) -> usize {
        self.bit_pos
    }

    /// 剩余位数（可为负，表示已越过声明长度）。
    pub fn bits_left(&self) -> isize {
        self.bits_total as isize - self.bit_pos as isize
    }

    /// 跳过 `n` 位。
    pub fn skip(&mut self, n: usize) {
        self.bit_pos = self.bit_pos.saturating_add(n);
    }

    /// 回退 `n` 位（仅内部窥视用）。
    pub fn rewind(&mut self, n: usize) {
        self.bit_pos = self.bit_pos.saturating_sub(n);
    }

    /// 读 1 位。
    pub fn read_bit(&mut self) -> Result<bool, BinkVideoError> {
        Ok(self.read_bits(1)? != 0)
    }

    /// 读 `n` 位（1..=32），LSB 优先拼成 `u32`。
    pub fn read_bits(&mut self, n: u32) -> Result<u32, BinkVideoError> {
        if n == 0 {
            return Ok(0);
        }
        if n > 32 {
            return Err(BinkVideoError::Msg(format!("一次最多读 32 位，请求 {n}")));
        }
        if self.bits_left() < n as isize {
            return Err(BinkVideoError::Msg(format!(
                "码流耗尽：需 {n} 位 · 剩 {}",
                self.bits_left()
            )));
        }

        let mut result: u64 = 0;
        let mut shift: u32 = 0;
        let mut remaining = n;
        while remaining > 0 {
            let byte_idx = self.bit_pos >> 3;
            let bit_in_byte = (self.bit_pos & 7) as u32;
            let take = (8 - bit_in_byte).min(remaining);
            let byte = self.data[byte_idx] as u64;
            let chunk = (byte >> bit_in_byte) & ((1u64 << take) - 1);
            result |= chunk << shift;
            shift += take;
            self.bit_pos += take as usize;
            remaining -= take;
        }
        Ok(result as u32)
    }

    /// 窥视最多 `n` 位（不足时高位补 0），不推进游标。
    pub fn peek_bits(&mut self, n: u32) -> Result<u32, BinkVideoError> {
        let have = self.bits_left();
        if have >= n as isize {
            let saved = self.bit_pos;
            let v = self.read_bits(n)?;
            self.bit_pos = saved;
            Ok(v)
        } else if have <= 0 {
            Ok(0)
        } else {
            let saved = self.bit_pos;
            let real = have as u32;
            let v = self.read_bits(real)?;
            self.bit_pos = saved;
            Ok(v)
        }
    }

    /// 对齐到下一个 32 位边界（平面段之间常用）。
    pub fn align_to_dword(&mut self) {
        let rem = self.bit_pos & 31;
        if rem != 0 {
            self.bit_pos += 32 - rem;
        }
    }
}

/// 平坦 Huffman 查表（最多 13 位前缀）。
#[derive(Debug, Clone)]
pub struct VlcTable {
    /// 每项：`(码长 << 16) | 符号`。
    entries: Vec<u32>,
    /// 前缀窥视宽度。
    bits: u32,
}

impl VlcTable {
    /// 由 16 组码字与码长构建；码字已按 LSB 优先布局。
    pub fn build(codes: &[u8; 16], lengths: &[u8; 16]) -> Result<Self, BinkVideoError> {
        let max_len = u32::from(*lengths.iter().max().unwrap_or(&0));
        if max_len == 0 || max_len > 13 {
            return Err(BinkVideoError::Msg(format!("VLC 码长越界：{max_len}")));
        }
        let bits = max_len;
        let size = 1usize << bits;
        let mut entries = vec![u32::MAX; size];
        for sym in 0..16u32 {
            let len = u32::from(lengths[sym as usize]);
            if len == 0 {
                continue;
            }
            let code = u32::from(codes[sym as usize]);
            let high_bits = bits - len;
            let entry = (len << 16) | sym;
            for high in 0..(1u32 << high_bits) {
                let idx = (code | (high << len)) as usize;
                entries[idx] = entry;
            }
        }
        if entries.iter().any(|&e| e == u32::MAX) {
            return Err(BinkVideoError::Msg("VLC 表存在空洞".into()));
        }
        Ok(Self { entries, bits })
    }

    /// 解码一个符号并按真实码长推进。
    pub fn decode(&self, reader: &mut BitReader<'_>) -> Result<u32, BinkVideoError> {
        let peek = reader.peek_bits(self.bits)?;
        let entry = self.entries[peek as usize];
        let len = entry >> 16;
        let sym = entry & 0xFFFF;
        reader.skip(len as usize);
        Ok(sym)
    }

    /// 前缀宽度。
    pub fn bits(&self) -> u32 {
        self.bits
    }
}

/// 预构建全部 16 棵固定树的 VLC 表。
pub fn build_fixed_vlc_tables() -> Result<[VlcTable; 16], BinkVideoError> {
    use super::bink_tables::{BINK_TREE_BITS, BINK_TREE_LENS};
    let mut out: [Option<VlcTable>; 16] = std::array::from_fn(|_| None);
    for t in 0..16 {
        out[t] = Some(VlcTable::build(&BINK_TREE_BITS[t], &BINK_TREE_LENS[t])?);
    }
    Ok(std::array::from_fn(|i| out[i].take().expect("filled")))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn read_single_bits_lsb_first() {
        let data = [0xA3u8];
        let mut r = BitReader::from_bytes(&data);
        assert_eq!(r.read_bit().unwrap(), true);
        assert_eq!(r.read_bit().unwrap(), true);
        assert_eq!(r.read_bit().unwrap(), false);
        assert_eq!(r.read_bit().unwrap(), false);
        assert_eq!(r.read_bit().unwrap(), false);
        assert_eq!(r.read_bit().unwrap(), true);
        assert_eq!(r.read_bit().unwrap(), false);
        assert_eq!(r.read_bit().unwrap(), true);
    }

    #[test]
    fn read_bits_nibbles() {
        let data = [0xA3u8];
        let mut r = BitReader::from_bytes(&data);
        assert_eq!(r.read_bits(4).unwrap(), 0x3);
        assert_eq!(r.read_bits(4).unwrap(), 0xA);
    }

    #[test]
    fn read_bits_across_bytes() {
        let data = [0x78u8, 0x56];
        let mut r = BitReader::from_bytes(&data);
        assert_eq!(r.read_bits(16).unwrap(), 0x5678);
    }

    #[test]
    fn align_to_dword() {
        let data = [0xFFu8; 8];
        let mut r = BitReader::from_bytes(&data);
        r.skip(5);
        r.align_to_dword();
        assert_eq!(r.pos(), 32);
    }

    #[test]
    fn eof_errors() {
        let data = [0u8; 1];
        let mut r = BitReader::from_bytes(&data);
        r.skip(8);
        assert!(r.read_bit().is_err());
    }

    #[test]
    fn fixed_vlc_tables_build() {
        let tables = build_fixed_vlc_tables().unwrap();
        assert_eq!(tables[0].bits(), 4);
        // 全 4 位等长树：读 0b0000 → 符号 0
        let data = [0x00u8];
        let mut r = BitReader::from_bytes(&data);
        assert_eq!(tables[0].decode(&mut r).unwrap(), 0);
    }
}
