//! Westwood CSV 行切分（逗号分隔；保留空列；不做 RFC4180 引号转义）。

use super::row::{CsvField, CsvRow};

/// 将一行文本切成 [`CsvRow`]。
///
/// - 仅按 `,` 分割（不按 `;`，与 INI 标量内嵌列表不同）
/// - 每列 `trim`；连续逗号保留空字段
/// - 空行 → 零字段行
pub fn parse_westwood_csv_line(raw: &str) -> CsvRow {
    let line = raw.trim_end_matches(['\r', '\n']);
    if line.is_empty() {
        return CsvRow::default();
    }
    let fields = line
        .split(',')
        .map(|part| CsvField {
            value: part.trim().to_string(),
        })
        .collect();
    CsvRow { fields }
}
