//! Rules / art 字段合并 schema（edition/mod 无关的默认策略表）。
//!
//! `ra-assets` 只执行合并；本模块声明哪些键是列表语义，哪些保持标量后写覆盖。

use ra_assets::{EntryMergePolicy, FieldMergeOverrides};

/// Techno 类型节字段合并覆盖（用于层叠 rules / MP / mod）。
///
/// 列表类键用 [`EntryMergePolicy::AppendValues`]；未列出的键走装载视图的节默认策略。
pub fn techno_section_field_overrides() -> FieldMergeOverrides {
    let mut overrides = FieldMergeOverrides::new();
    for key in [
        "Owner",
        "RequiredHouses",
        "ForbiddenHouses",
        "Prerequisite",
        "PrerequisiteOverride",
    ] {
        overrides.set(key, EntryMergePolicy::AppendValues);
    }
    overrides
}
