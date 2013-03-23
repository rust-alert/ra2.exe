//! HVA 体素动画：每帧每肢节的 3×4 变换矩阵。
//!
//! 布局：16 字节文件名 → `frame_count` / `section_count` → 节名表 → 矩阵表。

use ra_types::{RaError, RaResult};

const MIN_SIZE: usize = 24;
const SECTION_NAME_SIZE: usize = 16;
const MATRIX_FLOATS: usize = 12;

/// 解析后的 HVA。
#[derive(Debug, Clone)]
pub struct HvaFile {
    pub frame_count: u32,
    pub section_count: u32,
    pub section_names: Vec<String>,
    /// `[frame * section_count + section]` → 3×4 行主序矩阵。
    pub transforms: Vec<[f32; 12]>,
}

impl HvaFile {
    pub fn parse(data: &[u8]) -> RaResult<Self> {
        if data.len() < MIN_SIZE {
            return Err(RaError::Parse("hva 过小".into()));
        }
        let frame_count = read_u32(data, 16);
        let section_count = read_u32(data, 20);
        if section_count == 0 {
            return Err(RaError::Parse("hva 节数为 0".into()));
        }

        let names_end = MIN_SIZE + SECTION_NAME_SIZE * section_count as usize;
        if data.len() < names_end {
            return Err(RaError::Parse("hva 节名表截断".into()));
        }

        let mut section_names = Vec::with_capacity(section_count as usize);
        for i in 0..section_count as usize {
            let off = MIN_SIZE + i * SECTION_NAME_SIZE;
            section_names.push(read_c_string(&data[off..off + SECTION_NAME_SIZE]));
        }

        let total = frame_count as usize * section_count as usize;
        let matrix_bytes = total * MATRIX_FLOATS * 4;
        let matrix_start = names_end;
        if data.len() < matrix_start + matrix_bytes {
            return Err(RaError::Parse("hva 矩阵表截断".into()));
        }

        let mut transforms = Vec::with_capacity(total);
        let mut pos = matrix_start;
        for _ in 0..total {
            let mut m = [0.0f32; 12];
            for (k, slot) in m.iter_mut().enumerate() {
                *slot = read_f32(data, pos + k * 4);
            }
            transforms.push(m);
            pos += MATRIX_FLOATS * 4;
        }

        Ok(Self {
            frame_count,
            section_count,
            section_names,
            transforms,
        })
    }

    pub fn get_transform(&self, frame: u32, section: u32) -> Option<&[f32; 12]> {
        if frame >= self.frame_count || section >= self.section_count {
            return None;
        }
        let idx = frame as usize * self.section_count as usize + section as usize;
        self.transforms.get(idx)
    }
}

fn read_u32(data: &[u8], off: usize) -> u32 {
    u32::from_le_bytes(data[off..off + 4].try_into().unwrap())
}

fn read_f32(data: &[u8], off: usize) -> f32 {
    f32::from_le_bytes(data[off..off + 4].try_into().unwrap())
}

fn read_c_string(bytes: &[u8]) -> String {
    let end = bytes.iter().position(|&b| b == 0).unwrap_or(bytes.len());
    String::from_utf8_lossy(&bytes[..end]).into_owned()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn identity_row_major() -> [f32; 12] {
        [
            1.0, 0.0, 0.0, 0.0, //
            0.0, 1.0, 0.0, 0.0, //
            0.0, 0.0, 1.0, 0.0,
        ]
    }

    fn sample_hva() -> Vec<u8> {
        let mut data = Vec::new();
        data.extend_from_slice(b"test.hva\0\0\0\0\0\0\0\0");
        data.extend_from_slice(&2u32.to_le_bytes());
        data.extend_from_slice(&1u32.to_le_bytes());
        data.extend_from_slice(b"body\0\0\0\0\0\0\0\0\0\0\0\0");
        for f in identity_row_major() {
            data.extend_from_slice(&f.to_le_bytes());
        }
        // frame 1: translate (1,2,3)
        let mut m = identity_row_major();
        m[3] = 1.0;
        m[7] = 2.0;
        m[11] = 3.0;
        for f in m {
            data.extend_from_slice(&f.to_le_bytes());
        }
        data
    }

    #[test]
    fn parse_two_frames() {
        let hva = HvaFile::parse(&sample_hva()).unwrap();
        assert_eq!(hva.frame_count, 2);
        assert_eq!(hva.section_count, 1);
        assert_eq!(hva.section_names[0], "body");
        let t0 = hva.get_transform(0, 0).unwrap();
        assert_eq!(t0[0], 1.0);
        let t1 = hva.get_transform(1, 0).unwrap();
        assert_eq!(t1[3], 1.0);
        assert_eq!(t1[7], 2.0);
        assert_eq!(t1[11], 3.0);
    }

    #[test]
    fn reject_too_small() {
        assert!(HvaFile::parse(&[0u8; 8]).is_err());
    }
}
