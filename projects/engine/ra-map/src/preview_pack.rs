//! 地图 `[Preview]` / `[PreviewPack]`：大厅缩略图（LZO 分块 → 行优先 RGB24 → RGBA）。

use image::RgbaImage;
use ra_assets::{IniDocument, numbered_section_concat};
use ra_types::{RaError, RaResult};

use crate::{base64, lzo};

/// 从场景 INI 解出的预览缩略图。
#[derive(Debug, Clone)]
pub struct MapPreviewImage {
    /// 像素宽。
    pub width: u32,
    /// 像素高。
    pub height: u32,
    /// RGBA 像素（行优先，`width * height * 4`）。
    pub rgba: Vec<u8>,
}

impl MapPreviewImage {
    /// 转为 `image::RgbaImage`（缓冲长度须匹配）。
    pub fn into_rgba_image(self) -> Option<RgbaImage> {
        RgbaImage::from_raw(self.width, self.height, self.rgba)
    }
}

/// 解析 `[Preview] Size=`：`w,h` 或 `x,y,w,h`（取宽高）。
pub fn parse_preview_size(raw: &str) -> Option<(u32, u32)> {
    let parts: Vec<&str> = raw.split(',').map(str::trim).collect();
    match parts.as_slice() {
        [w, h] => Some((w.parse().ok()?, h.parse().ok()?)),
        [_, _, w, h, ..] => Some((w.parse().ok()?, h.parse().ok()?)),
        _ => None,
    }
}

/// 从已拼接的 PreviewPack base64 文本解码 RGBA。
pub fn decode_preview_pack(pack_b64: &str, width: u32, height: u32) -> RaResult<MapPreviewImage> {
    let encoded = pack_b64.trim();
    if encoded.is_empty() {
        return Err(RaError::Parse("PreviewPack 为空".into()));
    }
    if width == 0 || height == 0 {
        return Err(RaError::Parse("Preview Size 宽高为 0".into()));
    }
    let expected = (width as usize)
        .checked_mul(height as usize)
        .and_then(|p| p.checked_mul(3))
        .ok_or_else(|| RaError::Parse("Preview 尺寸溢出".into()))?;

    let compressed = base64::base64_decode(encoded).map_err(RaError::Parse)?;
    let rgb = lzo::decompress_chunks(&compressed).map_err(|e| RaError::Parse(e.to_string()))?;
    if rgb.len() != expected {
        return Err(RaError::Parse(format!("PreviewPack 字节数 {} 与期望 {} 不符", rgb.len(), expected)));
    }

    let mut rgba = Vec::with_capacity(expected / 3 * 4);
    for px in rgb.chunks_exact(3) {
        rgba.extend_from_slice(&[px[0], px[1], px[2], 255]);
    }
    Ok(MapPreviewImage { width, height, rgba })
}

/// 从场景 INI 文档解码预览；无 `[Preview]` / `[PreviewPack]` 时返回 `Ok(None)`。
pub fn decode_preview_from_ini(doc: &IniDocument) -> RaResult<Option<MapPreviewImage>> {
    let Some(size_raw) = doc.get("Preview", "Size")
    else {
        return Ok(None);
    };
    let Some((width, height)) = parse_preview_size(size_raw)
    else {
        return Err(RaError::Parse(format!("无效 [Preview] Size: {size_raw}")));
    };
    let Some(pack) = numbered_section_concat(doc, "PreviewPack")
    else {
        return Ok(None);
    };
    if pack.chars().all(|c| c.is_whitespace()) {
        return Ok(None);
    }
    decode_preview_pack(&pack, width, height).map(Some)
}

/// 从 `.map` / `.mpr` 原始字节解码大厅预览图。
pub fn decode_preview_from_map_bytes(bytes: &[u8]) -> RaResult<Option<MapPreviewImage>> {
    let doc = IniDocument::parse(bytes)?;
    decode_preview_from_ini(&doc)
}
