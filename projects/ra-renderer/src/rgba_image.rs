//! CPU 侧 RGBA 图像，供上传到 GPU 纹理。

/// 行主序 RGBA8 像素缓冲，宽高与 `pixels` 长度必须一致。
#[derive(Debug, Clone)]
pub struct RgbaImage {
    /// 图像宽度（像素）。
    pub width: u32,
    /// 图像高度（像素）。
    pub height: u32,
    /// 紧凑 RGBA 字节：`width * height * 4`。
    pub pixels: Vec<u8>,
}

impl RgbaImage {
    /// 校验尺寸与像素长度，合法则构造实例，否则返回 `None`。
    pub fn new(width: u32, height: u32, pixels: Vec<u8>) -> Option<Self> {
        let expected = (width as usize).checked_mul(height as usize)?.checked_mul(4)?;
        if pixels.len() != expected || width == 0 || height == 0 {
            return None;
        }
        Some(Self { width, height, pixels })
    }
}
