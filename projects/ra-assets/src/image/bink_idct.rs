//! Bink 1 自研 8×8 整数 IDCT（YUV 平面 → 后续 GPU 上传）。
//!
//! 变换核与扫描序来自公开 Bink 1 格式描述；实现为仓内自研，不链入 FFmpeg。

/// Bink DCT / residue 的 8×8 扫描序（格式常数）。
pub const BINK_SCAN: [u8; 64] = [
    0, 1, 8, 9, 2, 3, 10, 11, 4, 5, 12, 13, 6, 7, 14, 15, 20, 21, 28, 29, 22, 23, 30, 31, 16, 17, 24, 25, 32, 33, 40,
    41, 34, 35, 42, 43, 48, 49, 56, 57, 50, 51, 58, 59, 18, 19, 26, 27, 36, 37, 44, 45, 38, 39, 46, 47, 52, 53, 60, 61,
    54, 55, 62, 63,
];

const A1: i32 = 2896; // (1/√2)<<12
const A2: i32 = 2217;
const A3: i32 = 3784;
const A4: i32 = -5352;

#[inline]
fn mul(x: i32, y: i32) -> i32 {
    ((x as i64 * y as i64) >> 11) as i32
}

/// 一列或一行的 8 点 IDCT；`munge` 为恒等或 `((x)+0x7F)>>8`。
#[inline]
fn idct8(src: &[i32; 8], munge: impl Fn(i32) -> i32) -> [i32; 8] {
    let a0 = src[0] + src[4];
    let a1 = src[0] - src[4];
    let a2 = src[2] + src[6];
    let a3 = mul(A1, src[2] - src[6]);
    let a4 = src[5] + src[3];
    let a5 = src[5] - src[3];
    let a6 = src[1] + src[7];
    let a7 = src[1] - src[7];
    let b0 = a4 + a6;
    let b1 = mul(A3, a5 + a7);
    let b2 = mul(A4, a5) - b0 + b1;
    let b3 = mul(A1, a6 - a4) - b2;
    let b4 = mul(A2, a7) + b3 - b1;
    [
        munge(a0 + a2 + b0),
        munge(a1 + a3 - a2 + b2),
        munge(a1 - a3 + a2 + b3),
        munge(a0 - a2 - b4),
        munge(a0 - a2 + b4),
        munge(a1 - a3 + a2 - b3),
        munge(a1 + a3 - a2 - b2),
        munge(a0 + a2 - b0),
    ]
}

/// 列变换：AC 全 0 时直接广播 DC（不做缩放）。
#[inline]
fn idct_col(block: &[i32; 64], col: usize, temp: &mut [i32; 64]) {
    let src = [
        block[col],
        block[8 + col],
        block[16 + col],
        block[24 + col],
        block[32 + col],
        block[40 + col],
        block[48 + col],
        block[56 + col],
    ];
    if src[1] | src[2] | src[3] | src[4] | src[5] | src[6] | src[7] == 0 {
        let v = src[0];
        for row in 0..8 {
            temp[row * 8 + col] = v;
        }
        return;
    }
    let out = idct8(&src, |x| x);
    for row in 0..8 {
        temp[row * 8 + col] = out[row];
    }
}

/// 对 64 个 DCT 系数做 Bink 1 IDCT，原地写入行主序 8×8（已含行侧 `>>8` 舍入）。
pub fn bink_idct(block: &mut [i32; 64]) {
    let mut temp = [0i32; 64];
    for col in 0..8 {
        idct_col(block, col, &mut temp);
    }
    for row in 0..8 {
        let mut src = [0i32; 8];
        src.copy_from_slice(&temp[row * 8..row * 8 + 8]);
        let out = idct8(&src, |x| (x + 0x7F) >> 8);
        block[row * 8..row * 8 + 8].copy_from_slice(&out);
    }
}

/// IDCT 后写入目标平面（与公开实现一致：行变换结果直接落成字节，再裁剪）。
pub fn idct_put(dst: &mut [u8], stride: usize, block: &mut [i32; 64]) {
    let mut temp = [0i32; 64];
    for col in 0..8 {
        idct_col(block, col, &mut temp);
    }
    for row in 0..8 {
        let mut src = [0i32; 8];
        src.copy_from_slice(&temp[row * 8..row * 8 + 8]);
        let out = idct8(&src, |x| (x + 0x7F) >> 8);
        for col in 0..8 {
            dst[row * stride + col] = out[col].clamp(0, 255) as u8;
        }
    }
}

/// IDCT 后与参考块相加写入（帧间残差）。
pub fn idct_add(dst: &mut [u8], stride: usize, block: &mut [i32; 64]) {
    bink_idct(block);
    for row in 0..8 {
        for col in 0..8 {
            let i = row * stride + col;
            let v = i32::from(dst[i]) + block[row * 8 + col];
            dst[i] = v.clamp(0, 255) as u8;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn scan_visits_all_64() {
        let mut seen = [false; 64];
        for &i in &BINK_SCAN {
            seen[i as usize] = true;
        }
        assert!(seen.iter().all(|&x| x));
    }

    #[test]
    fn dc_only_idct_is_flat() {
        let mut block = [0i32; 64];
        block[0] = 1024;
        bink_idct(&mut block);
        let first = block[0];
        for &v in &block {
            assert_eq!(v, first);
        }
        // 仅 DC=1024：列广播后行变换得 (1024+127)>>8 == 4
        assert_eq!(first, 4);
    }

    #[test]
    fn idct_put_dc_fills_bytes() {
        let mut block = [0i32; 64];
        block[0] = 1024;
        let mut dst = [0u8; 64];
        idct_put(&mut dst, 8, &mut block);
        assert!(dst.iter().all(|&b| b == 4));
    }
}
