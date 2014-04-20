//! RGBA → PNG 编码（验收截图落盘）。

use std::{io::Cursor, path::Path};

use image::{ImageFormat, RgbaImage};
use ra_types::{RaError, RaResult};

/// 将 RGBA 图像编码为 PNG 字节。
pub fn encode_png(image: &RgbaImage) -> RaResult<Vec<u8>> {
    let mut out = Vec::new();
    image.write_to(&mut Cursor::new(&mut out), ImageFormat::Png).map_err(|e| RaError::Msg(format!("PNG 编码失败: {e}")))?;
    Ok(out)
}

/// 将 RGBA 图像写入 PNG 文件。
pub fn write_png_file(path: &Path, image: &RgbaImage) -> RaResult<()> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| RaError::Msg(format!("创建截图目录失败: {e}")))?;
    }
    image.save_with_format(path, ImageFormat::Png).map_err(|e| RaError::Msg(format!("PNG 写文件失败: {e}")))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn encode_png_emits_signature() {
        let img = RgbaImage::from_raw(2, 2, vec![255, 0, 0, 255, 0, 255, 0, 255, 0, 0, 255, 255, 255, 255, 0, 255]).unwrap();
        let bytes = encode_png(&img).unwrap();
        assert_eq!(&bytes[0..8], &[137, 80, 78, 71, 13, 10, 26, 10]);
    }
}
