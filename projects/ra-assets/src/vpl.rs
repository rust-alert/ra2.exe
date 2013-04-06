//! VPL：体素光照查表（亮度页 × 调色板索引 → 着色后索引）。

use ra_types::{RaError, RaResult};

const HEADER: usize = 16;
const PALETTE_BYTES: usize = 768;
const PAGE_SIZE: usize = 256;

/// 解析后的 VPL。
#[derive(Debug, Clone)]
pub struct VplFile {
    pub first_remap: u32,
    pub last_remap: u32,
    pub num_sections: u32,
    pages: Vec<[u8; PAGE_SIZE]>,
}

impl VplFile {
    /// 解析 `.vpl` 字节。
    pub fn parse(data: &[u8]) -> RaResult<Self> {
        if data.len() < HEADER + PALETTE_BYTES {
            return Err(RaError::Parse(format!(
                "vpl 过小：{} 字节",
                data.len()
            )));
        }
        let first_remap = read_u32(data, 0);
        let last_remap = read_u32(data, 4);
        let num_sections = read_u32(data, 8);
        if num_sections == 0 {
            return Err(RaError::Parse("vpl 节数为 0".into()));
        }
        let pages_start = HEADER + PALETTE_BYTES;
        let needed = pages_start + num_sections as usize * PAGE_SIZE;
        if data.len() < needed {
            return Err(RaError::Parse(format!(
                "vpl 截断：需 {needed} 字节，实有 {}",
                data.len()
            )));
        }

        let mut pages = Vec::with_capacity(num_sections as usize);
        for i in 0..num_sections as usize {
            let off = pages_start + i * PAGE_SIZE;
            let mut page = [0u8; PAGE_SIZE];
            page.copy_from_slice(&data[off..off + PAGE_SIZE]);
            pages.push(page);
        }

        Ok(Self {
            first_remap,
            last_remap,
            num_sections,
            pages,
        })
    }

    pub fn page_count(&self) -> usize {
        self.pages.len()
    }

    /// `(亮度页, 颜色索引) → 着色后的调色板索引`；页越界夹到末页。
    pub fn remap_color(&self, page: u8, color_index: u8) -> u8 {
        if self.pages.is_empty() {
            return color_index;
        }
        let page_idx = usize::from(page).min(self.pages.len() - 1);
        self.pages[page_idx][usize::from(color_index)]
    }

    /// 把法线索引粗映射到亮度页（预览用，非完整 Blinn-Phong 表）。
    pub fn page_from_normal(&self, normal_index: u8) -> u8 {
        let n = self.pages.len();
        if n <= 1 {
            return 0;
        }
        ((u32::from(normal_index) * (n as u32 - 1)) / 255) as u8
    }
}

fn read_u32(data: &[u8], off: usize) -> u32 {
    u32::from_le_bytes(data[off..off + 4].try_into().unwrap())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_vpl() -> Vec<u8> {
        let mut data = Vec::new();
        data.extend_from_slice(&16u32.to_le_bytes());
        data.extend_from_slice(&31u32.to_le_bytes());
        data.extend_from_slice(&2u32.to_le_bytes());
        data.extend_from_slice(&0u32.to_le_bytes());
        data.extend_from_slice(&[0u8; PALETTE_BYTES]);
        data.extend_from_slice(&(0..=255u8).collect::<Vec<_>>());
        data.extend_from_slice(&[42u8; PAGE_SIZE]);
        data
    }

    #[test]
    fn parse_two_pages() {
        let vpl = VplFile::parse(&sample_vpl()).unwrap();
        assert_eq!(vpl.first_remap, 16);
        assert_eq!(vpl.last_remap, 31);
        assert_eq!(vpl.page_count(), 2);
        assert_eq!(vpl.remap_color(0, 100), 100);
        assert_eq!(vpl.remap_color(1, 200), 42);
        assert_eq!(vpl.remap_color(9, 7), 42);
    }

    #[test]
    fn reject_tiny() {
        assert!(VplFile::parse(&[0u8; 8]).is_err());
    }
}
