//! INI：旧兼容解析 + 新通用文档 / oak 桥接。

mod document;
mod from_oak;
mod legacy;

pub use document::{IniEntry, IniSectionNode, ParsedIniDocument, SourceId, SourceSpan};
pub use from_oak::parse_with_oak;
pub use legacy::{IniDocument, IniSection};
