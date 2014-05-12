//! Bink 1 DCT 系数码流读取与反量化（自研，对接 IDCT）。

use super::{
    bink_bits::BitReader,
    bink_idct::BINK_SCAN,
    bink_quant::{BINK_INTER_QUANT, BINK_INTRA_QUANT},
    bink_video::BinkVideoError,
};

/// 从码流读取 AC 系数（DC 已写入 `block[0]`），返回量化档位 0..=15。
///
/// `q_override` 为 `Some` 时使用固定档位（旧版 BIKb）；BIKi/BIKk 传 `None` 从码流读 4 位。
pub fn read_dct_coeffs(
    r: &mut BitReader<'_>,
    block: &mut [i32; 64],
    coef_idx: &mut [usize; 64],
    coef_count: &mut usize,
    q_override: Option<u32>,
) -> Result<usize, BinkVideoError> {
    if r.bits_left() < 4 {
        return Err(BinkVideoError::Msg("DCT 系数码流过短".into()));
    }

    let mut coef_list = [0i32; 128];
    let mut mode_list = [0i32; 128];
    let mut list_start = 64usize;
    let mut list_end = 64usize;
    *coef_count = 0;

    // 初始工作列表：三组 mode0 起点 + 三个 mode3 低频槽。
    coef_list[list_end] = 4;
    mode_list[list_end] = 0;
    list_end += 1;
    coef_list[list_end] = 24;
    mode_list[list_end] = 0;
    list_end += 1;
    coef_list[list_end] = 44;
    mode_list[list_end] = 0;
    list_end += 1;
    coef_list[list_end] = 1;
    mode_list[list_end] = 3;
    list_end += 1;
    coef_list[list_end] = 2;
    mode_list[list_end] = 3;
    list_end += 1;
    coef_list[list_end] = 3;
    mode_list[list_end] = 3;
    list_end += 1;

    let mut bits = r.read_bits(4)? as i32 - 1;
    while bits >= 0 {
        let mut list_pos = list_start;
        while list_pos < list_end {
            if (mode_list[list_pos] | coef_list[list_pos]) == 0 || !r.read_bit()? {
                list_pos += 1;
                continue;
            }
            let mut ccoef = coef_list[list_pos];
            let mode = mode_list[list_pos];
            match mode {
                0 => {
                    coef_list[list_pos] = ccoef + 4;
                    mode_list[list_pos] = 1;
                    // fallthrough to case 2 body without the mode==2 clear
                    for _ in 0..4 {
                        if r.read_bit()? {
                            list_start -= 1;
                            coef_list[list_start] = ccoef;
                            mode_list[list_start] = 3;
                        } else {
                            let t = read_signed_coef(r, bits)?;
                            block[BINK_SCAN[ccoef as usize] as usize] = t;
                            coef_idx[*coef_count] = ccoef as usize;
                            *coef_count += 1;
                        }
                        ccoef += 1;
                    }
                }
                2 => {
                    coef_list[list_pos] = 0;
                    mode_list[list_pos] = 0;
                    list_pos += 1;
                    for _ in 0..4 {
                        if r.read_bit()? {
                            list_start -= 1;
                            coef_list[list_start] = ccoef;
                            mode_list[list_start] = 3;
                        } else {
                            let t = read_signed_coef(r, bits)?;
                            block[BINK_SCAN[ccoef as usize] as usize] = t;
                            coef_idx[*coef_count] = ccoef as usize;
                            *coef_count += 1;
                        }
                        ccoef += 1;
                    }
                }
                1 => {
                    mode_list[list_pos] = 2;
                    for _ in 0..3 {
                        ccoef += 4;
                        coef_list[list_end] = ccoef;
                        mode_list[list_end] = 2;
                        list_end += 1;
                    }
                }
                3 => {
                    let t = read_signed_coef(r, bits)?;
                    block[BINK_SCAN[ccoef as usize] as usize] = t;
                    coef_idx[*coef_count] = ccoef as usize;
                    *coef_count += 1;
                    coef_list[list_pos] = 0;
                    mode_list[list_pos] = 0;
                    list_pos += 1;
                }
                _ => {
                    return Err(BinkVideoError::Msg(format!("非法 DCT 列表模式 {mode}")));
                }
            }
        }
        bits -= 1;
    }

    let quant_idx = match q_override {
        Some(q) => {
            if q > 15 {
                return Err(BinkVideoError::Msg(format!("量化档越界 {q}")));
            }
            q as usize
        }
        None => r.read_bits(4)? as usize,
    };
    Ok(quant_idx)
}

#[inline]
fn read_signed_coef(r: &mut BitReader<'_>, bits: i32) -> Result<i32, BinkVideoError> {
    if bits == 0 {
        // 1 或 -1
        Ok(1 - ((r.read_bit()? as i32) << 1))
    } else {
        let mut t = r.read_bits(bits as u32)? as i32 | (1 << bits);
        let sign = -(r.read_bit()? as i32);
        t = (t ^ sign) - sign;
        Ok(t)
    }
}

/// 按量化档反量化 DC 与已读 AC。
pub fn unquantize_dct_coeffs(
    block: &mut [i32; 64],
    quant: &[i32; 64],
    coef_count: usize,
    coef_idx: &[usize; 64],
) {
    block[0] = ((block[0] as i64 * quant[0] as i64) >> 11) as i32;
    for i in 0..coef_count {
        let idx = coef_idx[i];
        let scan = BINK_SCAN[idx] as usize;
        block[scan] = ((block[scan] as i64 * quant[idx] as i64) >> 11) as i32;
    }
}

/// 读满一帧内 DCT 块（含 DC 已就位）并反量化。
pub fn decode_intra_dct_block(
    r: &mut BitReader<'_>,
    dc: i32,
) -> Result<[i32; 64], BinkVideoError> {
    let mut block = [0i32; 64];
    block[0] = dc;
    let mut coef_idx = [0usize; 64];
    let mut coef_count = 0usize;
    let q = read_dct_coeffs(r, &mut block, &mut coef_idx, &mut coef_count, None)?;
    unquantize_dct_coeffs(&mut block, &BINK_INTRA_QUANT[q], coef_count, &coef_idx);
    Ok(block)
}

/// 读满一帧间 DCT 残差块并反量化。
pub fn decode_inter_dct_block(
    r: &mut BitReader<'_>,
    dc: i32,
) -> Result<[i32; 64], BinkVideoError> {
    let mut block = [0i32; 64];
    block[0] = dc;
    let mut coef_idx = [0usize; 64];
    let mut coef_count = 0usize;
    let q = read_dct_coeffs(r, &mut block, &mut coef_idx, &mut coef_count, None)?;
    unquantize_dct_coeffs(&mut block, &BINK_INTER_QUANT[q], coef_count, &coef_idx);
    Ok(block)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn emptyish_dct_reads_quant_index() {
        // bits=0（4bit 全0 → bits=-1 跳过循环）+ quant=0
        let data = [0u8];
        let mut r = BitReader::from_bytes(&data);
        let mut block = [0i32; 64];
        block[0] = 100;
        let mut coef_idx = [0usize; 64];
        let mut coef_count = 0usize;
        let q = read_dct_coeffs(&mut r, &mut block, &mut coef_idx, &mut coef_count, None).unwrap();
        assert_eq!(q, 0);
        assert_eq!(coef_count, 0);
        assert_eq!(block[0], 100);
    }
}
