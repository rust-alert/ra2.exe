//! 地图 `[Preview]` / `[PreviewPack]`：大厅缩略图（LZO 分块 → 行优先 RGB24 → RGBA）。

use image::RgbaImage;
use ra_assets::{IniDocument, from_csv_row, parse_westwood_csv_line};
use ra_types::{RaError, RaResult};
use serde::Deserialize;
use serde::de::Deserializer;

use crate::{base64, lzo, numbered_pack::try_decode_numbered_base64_pack};

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

#[derive(Debug, Deserialize)]
struct PreviewSizeWh {
    width: u32,
    height: u32,
}

#[derive(Debug, Deserialize)]
struct PreviewSizeXywh {
    _x: i32,
    _y: i32,
    width: u32,
    height: u32,
}

/// `[Preview]` 节字段（`Size` 在反序列化时解码为宽高）。
#[derive(Debug, Default, Deserialize)]
pub(crate) struct PreviewSectionFields {
    #[serde(rename = "Size", default, deserialize_with = "de_opt_preview_size")]
    pub size: Option<(u32, u32)>,
}

fn de_opt_preview_size<'de, D>(deserializer: D) -> Result<Option<(u32, u32)>, D::Error>
where
    D: Deserializer<'de>,
{
    let raw = String::deserialize(deserializer)?;
    Ok(parse_preview_size(&raw))
}

/// 解析 `[Preview] Size=`：`w,h` 或 `x,y,w,h`（取宽高）。
pub fn parse_preview_size(raw: &str) -> Option<(u32, u32)> {
    let row = parse_westwood_csv_line(raw);
    match row.len() {
        2 => {
            let parsed: PreviewSizeWh = from_csv_row(&row).ok()?;
            Some((parsed.width, parsed.height))
        }
        n if n >= 4 => {
            let parsed: PreviewSizeXywh = from_csv_row(&row).ok()?;
            Some((parsed.width, parsed.height))
        }
        _ => None,
    }
}

/// 从已拼接的 PreviewPack base64 文本解码 RGBA。
pub fn decode_preview_pack(pack_b64: &str, width: u32, height: u32) -> RaResult<MapPreviewImage> {
    let encoded = pack_b64.trim();
    if encoded.is_empty() {
        return Err(RaError::Parse("PreviewPack 为空".into()));
    }
    let compressed = base64::base64_decode(encoded).map_err(RaError::Parse)?;
    decode_preview_pack_bytes(&compressed, width, height)
}

/// 从 PreviewPack 压缩字节解码 RGBA。
pub fn decode_preview_pack_bytes(compressed: &[u8], width: u32, height: u32) -> RaResult<MapPreviewImage> {
    if width == 0 || height == 0 {
        return Err(RaError::Parse("Preview Size 宽高为 0".into()));
    }
    let expected = (width as usize)
        .checked_mul(height as usize)
        .and_then(|p| p.checked_mul(3))
        .ok_or_else(|| RaError::Parse("Preview 尺寸溢出".into()))?;

    let rgb = lzo::decompress_chunks(compressed).map_err(|e| RaError::Parse(e.to_string()))?;
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
    let Some(sec) = doc.section("Preview")
    else {
        return Ok(None);
    };
    let Some(size_raw) = sec.get("Size")
    else {
        return Ok(None);
    };
    let Some((width, height)) = parse_preview_size(size_raw)
    else {
        return Err(RaError::Parse(format!("无效 [Preview] Size: {size_raw}")));
    };
    let Some(compressed) = try_decode_numbered_base64_pack(doc, "PreviewPack")?
    else {
        return Ok(None);
    };
    decode_preview_pack_bytes(&compressed, width, height).map(Some)
}

/// 从 `.map` / `.mpr` 原始字节解码大厅预览图。
pub fn decode_preview_from_map_bytes(bytes: &[u8]) -> RaResult<Option<MapPreviewImage>> {
    let doc = IniDocument::parse(bytes)?;
    decode_preview_from_ini(&doc)
}
