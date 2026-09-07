//! RGBA → PNG 编码（验收截图落盘）。

use std::{fs::File, io::BufWriter, path::Path};

use ra_types::{RaError, RaResult};

use crate::rgba_image::RgbaImage;

/// 将 RGBA 图像编码为 PNG 字节。
pub fn encode_png(image: &RgbaImage) -> RaResult<Vec<u8>> {
    let mut out = Vec::new();
    {
        let mut encoder = png::Encoder::new(&mut out, image.width, image.height);
        encoder.set_color(png::ColorType::Rgba);
        encoder.set_depth(png::BitDepth::Eight);
        let mut writer = encoder
            .write_header()
            .map_err(|e| RaError::Msg(format!("PNG 写头失败: {e}")))?;
        writer
            .write_image_data(&image.pixels)
            .map_err(|e| RaError::Msg(format!("PNG 写像素失败: {e}")))?;
    }
    Ok(out)
}

/// 将 RGBA 图像写入 PNG 文件。
pub fn write_png_file(path: &Path, image: &RgbaImage) -> RaResult<()> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| RaError::Msg(format!("创建截图目录失败: {e}")))?;
    }
    let file = File::create(path).map_err(|e| RaError::Msg(format!("创建截图文件失败: {e}")))?;
    let mut encoder = png::Encoder::new(BufWriter::new(file), image.width, image.height);
    encoder.set_color(png::ColorType::Rgba);
    encoder.set_depth(png::BitDepth::Eight);
    let mut writer = encoder
        .write_header()
        .map_err(|e| RaError::Msg(format!("PNG 写头失败: {e}")))?;
    writer
        .write_image_data(&image.pixels)
        .map_err(|e| RaError::Msg(format!("PNG 写像素失败: {e}")))?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn encode_png_emits_signature() {
        let img = RgbaImage::new(2, 2, vec![255, 0, 0, 255, 0, 255, 0, 255, 0, 0, 255, 255, 255, 255, 0, 255]).unwrap();
        let bytes = encode_png(&img).unwrap();
        assert_eq!(&bytes[0..8], &[137, 80, 78, 71, 13, 10, 26, 10]);
    }
}
