//! 最小 INI 读取，用于 rules/art/地图启动。

use std::collections::BTreeMap;

use ra_types::{RaError, RaResult};

/// 单个 INI 节：键值表与写入顺序。
#[derive(Debug, Default, Clone)]
pub struct IniSection {
    /// 键 → 值（后写覆盖先写）。
    pub values: BTreeMap<String, String>,
    /// 写入顺序，供 IsoMapPack 等按行拼接使用。
    pub order: Vec<(String, String)>,
}

/// 完整 INI 文档（节名 → 节内容）。
#[derive(Debug, Default, Clone)]
pub struct IniDocument {
    /// 所有节（含无标题前导节，键为空串）。
    pub sections: BTreeMap<String, IniSection>,
}

impl IniDocument {
    /// 解析 UTF-8 INI 文本字节；`;` 起为行注释。
    pub fn parse(bytes: &[u8]) -> RaResult<Self> {
        let text = std::str::from_utf8(bytes).map_err(|e| RaError::Parse(e.to_string()))?;
        let mut doc = IniDocument::default();
        let mut current = String::from("");
        doc.sections.entry(current.clone()).or_default();

        for raw in text.lines() {
            let line = raw.split(';').next().unwrap_or("").trim();
            if line.is_empty() {
                continue;
            }
            if line.starts_with('[') && line.ends_with(']') {
                current = line[1..line.len() - 1].trim().to_string();
                doc.sections.entry(current.clone()).or_default();
                continue;
            }
            if let Some((k, v)) = line.split_once('=') {
                let key = k.trim().to_string();
                let val = v.trim().to_string();
                let section = doc.sections.entry(current.clone()).or_default();
                section.order.push((key.clone(), val.clone()));
                section.values.insert(key, val);
            }
        }
        Ok(doc)
    }

    /// 读取 `section` 下 `key` 的值。
    pub fn get(&self, section: &str, key: &str) -> Option<&str> {
        self.sections.get(section).and_then(|s| s.values.get(key)).map(|s| s.as_str())
    }

    /// 将编号键（`1=` / `2=` …）按数值序拼接，用于 IsoMapPack5 / OverlayPack。
    pub fn numbered_section_concat(&self, section: &str) -> Option<String> {
        let sec = self.sections.get(section)?;
        let mut pairs: Vec<(u32, &str)> = Vec::new();
        for (k, v) in &sec.order {
            if let Ok(n) = k.parse::<u32>() {
                pairs.push((n, v.as_str()));
            }
        }
        if pairs.is_empty() {
            return None;
        }
        pairs.sort_by_key(|(n, _)| *n);
        let mut out = String::new();
        for (_, v) in pairs {
            out.push_str(v);
        }
        Some(out)
    }
}
