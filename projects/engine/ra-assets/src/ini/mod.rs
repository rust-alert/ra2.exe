//! INI：Westwood 方言 → 通用文档；可选 Serde 字段反序列化。

mod document;
pub mod de;
pub mod parse;
mod value;

pub use de::{IniDeError, from_section};
pub use document::{IniDocument, IniEntry, IniSection, SourceId, SourceSpan};
pub use value::IniValue;
