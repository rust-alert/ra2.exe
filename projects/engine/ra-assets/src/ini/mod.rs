//! INI：Westwood 方言 → 通用文档。

mod document;
pub mod parse;

pub use document::{IniDocument, IniEntry, IniSection, SourceId, SourceSpan};
