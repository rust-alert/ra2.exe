//! 地图编号 pack 节：编号键拼接 → base64 → 压缩字节。
//!
//! IsoMapPack5 / OverlayPack / PreviewPack 等把 payload 拆成 `1=` / `2=` …，
//! base64 必须先拼文本再解码（不可按键分别解码）。

use ra_assets::{IniDocument, IniMergePolicy, EntryMergePolicy, LayeredIniView};
use ra_types::{RaError, RaResult};

use crate::base64;

fn numbered_pack_b64(doc: &IniDocument, section: &str) -> Option<String> {
    let policy = IniMergePolicy {
        default_entry: EntryMergePolicy::NumberedPack,
    };
    let docs = std::slice::from_ref(doc);
    LayeredIniView::new(docs, &policy).numbered_pack_concat(section)
}

/// 读取编号 pack 节并解码为压缩字节；缺节或空内容报错。
pub fn decode_numbered_base64_pack(doc: &IniDocument, section: &str) -> RaResult<Vec<u8>> {
    let b64 = numbered_pack_b64(doc, section).ok_or_else(|| RaError::Parse(format!("缺少 [{section}]")))?;
    let trimmed = b64.trim();
    if trimmed.is_empty() {
        return Err(RaError::Parse(format!("[{section}] 为空")));
    }
    base64::base64_decode(trimmed).map_err(RaError::Parse)
}

/// 可选读取编号 pack；缺节或全空白返回 `Ok(None)`。
pub fn try_decode_numbered_base64_pack(doc: &IniDocument, section: &str) -> RaResult<Option<Vec<u8>>> {
    let Some(b64) = numbered_pack_b64(doc, section)
    else {
        return Ok(None);
    };
    if b64.chars().all(|c| c.is_whitespace()) {
        return Ok(None);
    }
    base64::base64_decode(b64.trim()).map(Some).map_err(RaError::Parse)
}
