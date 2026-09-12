//! Westwood CSV 方言：行切分 + Serde 按列序反序列化。
//!
//! 与 [`crate::ini`] 同构：自研方言（非 RFC4180），标量语义与 INI 字段一致
//! （`yes`/`no`、trim、空串 → `Option::None`）。
//!
//! **列绑定**：结构体按**字段声明顺序**对应第 0、1、2… 列；字段名只用于诊断，
//! 不按名字查找列。多余列忽略；缺列时依赖 `#[serde(default)]` / `Option`。

mod de;
mod parse;
mod row;

pub use de::{CsvDeError, from_csv_row, from_row};
pub use parse::parse_westwood_csv_line;
pub use row::{CsvField, CsvRow};
