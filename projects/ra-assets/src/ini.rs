//! 最小 INI 读取，用于 rules/art 启动。

use std::collections::BTreeMap;

use ra_types::{RaError, RaResult};

#[derive(Debug, Default, Clone)]
pub struct IniSection {
    pub values: BTreeMap<String, String>,
}

#[derive(Debug, Default, Clone)]
pub struct IniDocument {
    pub sections: BTreeMap<String, IniSection>,
}

impl IniDocument {
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
                doc.sections
                    .entry(current.clone())
                    .or_default()
                    .values
                    .insert(k.trim().to_string(), v.trim().to_string());
            }
        }
        Ok(doc)
    }

    pub fn get(&self, section: &str, key: &str) -> Option<&str> {
        self.sections
            .get(section)
            .and_then(|s| s.values.get(key))
            .map(|s| s.as_str())
    }
}
