//! Bink 运动补偿后的 8×8 residue 码流（自研）。

use super::{bink_bits::BitReader, bink_idct::BINK_SCAN, bink_video::BinkVideoError};

/// 读取 residual 掩码位平面，写入 `block`（扫描序位置）。
pub fn read_residue(r: &mut BitReader<'_>, block: &mut [i16; 64], mut masks_count: i32) -> Result<(), BinkVideoError> {
    let mut coef_list = [0i32; 128];
    let mut mode_list = [0i32; 128];
    let mut list_start = 64usize;
    let mut list_end = 64usize;
    let mut nz_coeff = [0usize; 64];
    let mut nz_coeff_count = 0usize;

    coef_list[list_end] = 4;
    mode_list[list_end] = 0;
    list_end += 1;
    coef_list[list_end] = 24;
    mode_list[list_end] = 0;
    list_end += 1;
    coef_list[list_end] = 44;
    mode_list[list_end] = 0;
    list_end += 1;
    coef_list[list_end] = 0;
    mode_list[list_end] = 2;
    list_end += 1;

    let mut mask = 1i16 << r.read_bits(3)?;
    while mask != 0 {
        for i in 0..nz_coeff_count {
            if !r.read_bit()? {
                continue;
            }
            if block[nz_coeff[i]] < 0 {
                block[nz_coeff[i]] -= mask;
            }
            else {
                block[nz_coeff[i]] += mask;
            }
            masks_count -= 1;
            if masks_count < 0 {
                return Ok(());
            }
        }

        let mut list_pos = list_start;
        while list_pos < list_end {
            if (coef_list[list_pos] | mode_list[list_pos]) == 0 || !r.read_bit()? {
                list_pos += 1;
                continue;
            }
            let mut ccoef = coef_list[list_pos];
            let mode = mode_list[list_pos];
            match mode {
                0 => {
                    coef_list[list_pos] = ccoef + 4;
                    mode_list[list_pos] = 1;
                    for _ in 0..4 {
                        if r.read_bit()? {
                            list_start -= 1;
                            coef_list[list_start] = ccoef;
                            mode_list[list_start] = 3;
                        }
                        else {
                            let scan = BINK_SCAN[ccoef as usize] as usize;
                            nz_coeff[nz_coeff_count] = scan;
                            nz_coeff_count += 1;
                            let sign = -(r.read_bit()? as i16);
                            block[scan] = (mask ^ sign) - sign;
                            masks_count -= 1;
                            if masks_count < 0 {
                                return Ok(());
                            }
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
                        }
                        else {
                            let scan = BINK_SCAN[ccoef as usize] as usize;
                            nz_coeff[nz_coeff_count] = scan;
                            nz_coeff_count += 1;
                            let sign = -(r.read_bit()? as i16);
                            block[scan] = (mask ^ sign) - sign;
                            masks_count -= 1;
                            if masks_count < 0 {
                                return Ok(());
                            }
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
                    let scan = BINK_SCAN[ccoef as usize] as usize;
                    nz_coeff[nz_coeff_count] = scan;
                    nz_coeff_count += 1;
                    let sign = -(r.read_bit()? as i16);
                    block[scan] = (mask ^ sign) - sign;
                    coef_list[list_pos] = 0;
                    mode_list[list_pos] = 0;
                    list_pos += 1;
                    masks_count -= 1;
                    if masks_count < 0 {
                        return Ok(());
                    }
                }
                _ => return Err(BinkVideoError::Msg(format!("非法 residue 模式 {mode}"))),
            }
        }
        mask >>= 1;
    }
    Ok(())
}

/// 将 8×8 `i16` residual 加到平面块（裁剪到 0..=255）。
pub fn add_pixels8(dst: &mut [u8], stride: usize, block: &[i16; 64]) {
    for row in 0..8 {
        for col in 0..8 {
            let i = row * stride + col;
            let v = i32::from(dst[i]) + i32::from(block[row * 8 + col]);
            dst[i] = v.clamp(0, 255) as u8;
        }
    }
}
