//! 地图编号 pack 节：编号键分片 → base64 → 压缩字节。
//!
//! IsoMapPack5 / OverlayPack / PreviewPack 等把 payload 拆成 `1=` / `2=` …，
//! base64 必须按索引序拼文本再解码（不可按键分别解码）。解码入口吃分片，不先拼公开 `String`。

use ra_assets::{IniDocument, numbered_section_parts};
use ra_types::{RaError, RaResult};

use crate::base64;

fn parts_are_blank(parts: &[&str]) -> bool {
    parts.iter().all(|p| p.chars().all(|c| c.is_whitespace()))
}

/// 读取编号 pack 节并解码为压缩字节；缺节或空内容报错。
pub fn decode_numbered_base64_pack(doc: &IniDocument, section: &str) -> RaResult<Vec<u8>> {
    let parts = numbered_section_parts(doc, section).ok_or_else(|| RaError::Parse(format!("缺少 [{section}]")))?;
    if parts_are_blank(&parts) {
        return Err(RaError::Parse(format!("[{section}] 为空")));
    }
    base64::base64_decode_parts(parts).map_err(RaError::Parse)
}

/// 可选读取编号 pack；缺节或全空白返回 `Ok(None)`。
pub fn try_decode_numbered_base64_pack(doc: &IniDocument, section: &str) -> RaResult<Option<Vec<u8>>> {
    let Some(parts) = numbered_section_parts(doc, section)
    else {
        return Ok(None);
    };
    if parts_are_blank(&parts) {
        return Ok(None);
    }
    base64::base64_decode_parts(parts).map(Some).map_err(RaError::Parse)
}
