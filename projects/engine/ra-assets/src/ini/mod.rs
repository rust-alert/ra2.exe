//! INI：`oak-ini` AST → 通用文档。

mod document;
mod from_oak;

pub use document::{IniDocument, IniEntry, IniSection, SourceId, SourceSpan};
