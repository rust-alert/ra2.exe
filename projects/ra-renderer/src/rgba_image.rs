//! CPU 侧 RGBA 图像，供上传到 GPU 纹理。

#[derive(Debug, Clone)]
pub struct RgbaImage {
    pub width: u32,
    pub height: u32,
    pub pixels: Vec<u8>,
}

impl RgbaImage {
    pub fn new(width: u32, height: u32, pixels: Vec<u8>) -> Option<Self> {
        let expected = (width as usize).checked_mul(height as usize)?.checked_mul(4)?;
        if pixels.len() != expected || width == 0 || height == 0 {
            return None;
        }
        Some(Self {
            width,
            height,
            pixels,
        })
    }
}
