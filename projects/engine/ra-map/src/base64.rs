//! Base64 编解码（IsoMapPack5 / OverlayPack 等地图二进制段）。
//!
//! 使用标准字母表。地图文件把数据拆在编号 INI 键上，解码前会去掉空白。

use base64::{Engine, engine::general_purpose::STANDARD};

/// 将多段 base64 文本按序拼接并解码（跳过各段空白与换行）。
///
/// 供编号 pack 分片路径使用，避免调用方先 `String` 拼接再解码。
pub fn base64_decode_parts<'a>(parts: impl IntoIterator<Item = &'a str>) -> Result<Vec<u8>, String> {
    let mut cleaned = String::new();
    for part in parts {
        for c in part.chars() {
            if !c.is_ascii_whitespace() {
                cleaned.push(c);
            }
        }
    }
    if cleaned.is_empty() {
        return Ok(Vec::new());
    }
    STANDARD.decode(cleaned.as_bytes()).map_err(|e| e.to_string())
}

/// 将 base64 字符串解码为原始字节。
///
/// 跳过空白与换行（地图把数据拆在编号 INI 行上）。遇非法字符返回错误。
pub fn base64_decode(input: &str) -> Result<Vec<u8>, String> {
    base64_decode_parts(std::iter::once(input))
}

/// 编码为标准 base64（含 `=` 填充），供往返校验与测试构造。
pub fn base64_encode(input: &[u8]) -> String {
    STANDARD.encode(input)
}
