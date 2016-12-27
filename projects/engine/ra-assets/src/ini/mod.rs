//! INI：Westwood 方言 → 通用文档；可选 Serde 字段反序列化。

mod asset_refs;
mod document;
pub mod de;
pub mod merge;
mod numbered;
pub mod parse;
pub mod soft;
mod value;

pub use asset_refs::collect_shp_refs;
pub use de::{IniDeError, from_layered_section, from_section};
pub use document::{IniDocument, IniEntry, IniSection, SourceId, SourceSpan};
pub use merge::{
    EntryMergePolicy, FieldMergeOverrides, IniMergePolicy, LayeredIniView, LayeredSectionView, ResolvedIniValue,
    materialize_ini_layers,
};
pub use numbered::{concat_numbered_values, numbered_pairs, numbered_section_concat, numbered_section_parts, parse_numbered_key};
pub use soft::{deserialize_opt_bool, deserialize_opt_f32, deserialize_opt_f64, deserialize_opt_i32, deserialize_opt_u32};
pub use value::IniValue;
