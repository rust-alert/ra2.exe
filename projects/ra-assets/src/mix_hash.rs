//! MIX 文件名哈希（TS / RA2 CRC-32 + Westwood 填充）。

const CRC32_POLYNOMIAL: u32 = 0xEDB88320;

const CRC32_TABLE: [u32; 256] = {
    let mut table = [0u32; 256];
    let mut i = 0;
    while i < 256 {
        let mut crc = i as u32;
        let mut j = 0;
        while j < 8 {
            if crc & 1 != 0 {
                crc = (crc >> 1) ^ CRC32_POLYNOMIAL;
            } else {
                crc >>= 1;
            }
            j += 1;
        }
        table[i] = crc;
        i += 1;
    }
    table
};

/// 计算 MIX 条目 ID（大写文件名 + 填充后的 CRC-32）。
pub fn mix_hash(name: &str) -> i32 {
    let upper: Vec<u8> = name.bytes().map(|b| b.to_ascii_uppercase()).collect();
    let padded = westwood_pad(&upper);
    crc32(&padded) as i32
}

fn westwood_pad(data: &[u8]) -> Vec<u8> {
    let len = data.len();
    let residue = len % 4;
    if residue == 0 {
        return data.to_vec();
    }
    let padding_count = 4 - residue;
    let rounded_pos = len - residue;
    let fill_char = data[rounded_pos];
    let mut padded = Vec::with_capacity(len + padding_count);
    padded.extend_from_slice(data);
    padded.push(residue as u8);
    for _ in 1..padding_count {
        padded.push(fill_char);
    }
    padded
}

fn crc32(data: &[u8]) -> u32 {
    let mut crc = 0xFFFFFFFFu32;
    for &byte in data {
        let index = ((crc ^ u32::from(byte)) & 0xFF) as usize;
        crc = (crc >> 8) ^ CRC32_TABLE[index];
    }
    crc ^ 0xFFFFFFFF
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn crc32_known_vector() {
        assert_eq!(crc32(b"123456789"), 0xCBF43926);
    }

    #[test]
    fn mix_hash_case_insensitive() {
        assert_eq!(mix_hash("rules.ini"), mix_hash("RULES.INI"));
    }

    #[test]
    fn padding_for_rules_ini() {
        let padded = westwood_pad(b"RULES.INI");
        assert_eq!(padded.len(), 12);
        assert_eq!(padded[9], 0x01);
        assert_eq!(padded[10], b'I');
        assert_eq!(padded[11], b'I');
    }
}
