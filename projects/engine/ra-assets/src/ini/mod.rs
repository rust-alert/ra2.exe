//! INI：Westwood 方言 → 通用文档；可选 Serde 字段反序列化。

mod asset_refs;
mod document;
pub mod de;
pub mod merge;
mod numbered;
pub mod parse;
mod value;

pub use asset_refs::collect_shp_refs;
pub use de::{IniDeError, from_layered_section, from_section};
pub use document::{IniDocument, IniEntry, IniSection, SourceId, SourceSpan};
pub use merge::{
    EntryMergePolicy, FieldMergeOverrides, IniMergePolicy, LayeredIniView, LayeredSectionView, ResolvedIniValue,
};
pub use numbered::{concat_numbered_values, numbered_section_concat};
pub use value::IniValue;
