//! 配置来源、合并与诊断。
//!
//! 本 crate 不解释游戏语义；adaptor 决定读哪些文件及含义。

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

/// 一条配置诊断。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConfigDiagnostic {
    pub source: String,
    pub message: String,
}

/// 扁平字符串配置表（桌面设置等够用；INI 游戏语义不在此解释）。
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ConfigTable {
    values: BTreeMap<String, String>,
}

impl ConfigTable {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn get(&self, key: &str) -> Option<&str> {
        self.values.get(key).map(|s| s.as_str())
    }

    pub fn insert(&mut self, key: impl Into<String>, value: impl Into<String>) {
        self.values.insert(key.into(), value.into());
    }

    pub fn iter(&self) -> impl Iterator<Item = (&str, &str)> {
        self.values.iter().map(|(k, v)| (k.as_str(), v.as_str()))
    }
}

/// 带来源标签的一层配置。
#[derive(Debug, Clone)]
pub struct ConfigLayer {
    pub label: String,
    pub table: ConfigTable,
}

/// 合并结果：后者覆盖前者同名键。
#[derive(Debug, Clone, Default)]
pub struct MergedConfig {
    pub table: ConfigTable,
    pub diagnostics: Vec<ConfigDiagnostic>,
}

impl MergedConfig {
    pub fn merge_layers(layers: &[ConfigLayer]) -> Self {
        let mut out = Self::default();
        for layer in layers {
            for (k, v) in layer.table.iter() {
                out.table.insert(k, v);
            }
        }
        out
    }

    pub fn get(&self, key: &str) -> Option<&str> {
        self.table.get(key)
    }
}

/// 解析极简 TOML 风格键值（`key = "value"`，`#` 注释）。不支持表/数组。
pub fn parse_kv_toml_lite(text: &str, source_label: &str) -> (ConfigTable, Vec<ConfigDiagnostic>) {
    let mut table = ConfigTable::new();
    let mut diagnostics = Vec::new();
    for (lineno, raw) in text.lines().enumerate() {
        let line = raw.split('#').next().unwrap_or("").trim();
        if line.is_empty() || line.starts_with('[') {
            continue;
        }
        let Some((k, v)) = line.split_once('=') else {
            diagnostics.push(ConfigDiagnostic {
                source: format!("{source_label}:{lineno}"),
                message: format!("无法解析行: {raw}"),
            });
            continue;
        };
        let key = k.trim();
        if key.is_empty() {
            diagnostics.push(ConfigDiagnostic {
                source: format!("{source_label}:{lineno}"),
                message: "空键".into(),
            });
            continue;
        }
        let val = v.trim().trim_matches('"').trim_matches('\'');
        table.insert(key, val);
    }
    (table, diagnostics)
}

/// 从候选路径加载第一份存在的文本文件。
pub fn read_first_existing(candidates: &[&Path]) -> Option<(PathBuf, String)> {
    for path in candidates {
        if let Ok(text) = std::fs::read_to_string(path) {
            return Some((path.to_path_buf(), text));
        }
    }
    None
}

/// 桌面启动设置（由合并后的键值填充）。
#[derive(Debug, Clone)]
pub struct DesktopSettings {
    pub ra2_dir: PathBuf,
    pub edition: Option<String>,
    /// 预留：目标战网接入地址（协议未定点前仅配置，不接 socket）。
    pub net_url: Option<String>,
    pub net_room: Option<String>,
}

impl Default for DesktopSettings {
    fn default() -> Self {
        Self {
            ra2_dir: PathBuf::from("."),
            edition: None,
            net_url: None,
            net_room: None,
        }
    }
}

impl DesktopSettings {
    pub fn from_merged(merged: &MergedConfig) -> Self {
        let mut s = Self::default();
        if let Some(v) = merged.get("ra2_dir").or_else(|| merged.get("game_dir")) {
            s.ra2_dir = PathBuf::from(v);
        }
        if let Some(v) = merged.get("edition").filter(|v| !v.is_empty()) {
            s.edition = Some(v.to_string());
        }
        if let Some(v) = merged
            .get("net_url")
            .or_else(|| merged.get("battlenet_url"))
            .filter(|v| !v.is_empty())
        {
            s.net_url = Some(v.to_string());
        }
        if let Some(v) = merged
            .get("net_room")
            .or_else(|| merged.get("room"))
            .filter(|v| !v.is_empty())
        {
            s.net_room = Some(v.to_string());
        }
        s
    }

    /// 加载桌面配置：默认 ← 文件覆盖。返回设置与诊断。
    pub fn load_or_default() -> (Self, Vec<ConfigDiagnostic>) {
        let defaults = ConfigLayer {
            label: "defaults".into(),
            table: {
                let mut t = ConfigTable::new();
                t.insert("ra2_dir", ".");
                t
            },
        };
        let mut layers = vec![defaults];
        let mut diagnostics = Vec::new();
        if let Some((path, text)) =
            read_first_existing(&[Path::new("config.toml"), Path::new("ra2.toml")])
        {
            let label = path.display().to_string();
            let (table, mut diags) = parse_kv_toml_lite(&text, &label);
            diagnostics.append(&mut diags);
            layers.push(ConfigLayer { label, table });
        }
        let mut merged = MergedConfig::merge_layers(&layers);
        merged.diagnostics.append(&mut diagnostics);
        let settings = Self::from_merged(&merged);
        (settings, merged.diagnostics)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn later_layer_overrides() {
        let mut a = ConfigTable::new();
        a.insert("ra2_dir", ".");
        let mut b = ConfigTable::new();
        b.insert("ra2_dir", "C:/games/ra2");
        b.insert("edition", "yr");
        let merged = MergedConfig::merge_layers(&[
            ConfigLayer {
                label: "a".into(),
                table: a,
            },
            ConfigLayer {
                label: "b".into(),
                table: b,
            },
        ]);
        let s = DesktopSettings::from_merged(&merged);
        assert_eq!(s.ra2_dir, PathBuf::from("C:/games/ra2"));
        assert_eq!(s.edition.as_deref(), Some("yr"));
    }

    #[test]
    fn parse_kv_skips_comments() {
        let (t, d) = parse_kv_toml_lite(
            "# hi\nra2_dir = \"D:/RA2\"\nedition = 'yr'\n",
            "t",
        );
        assert!(d.is_empty());
        assert_eq!(t.get("ra2_dir"), Some("D:/RA2"));
        assert_eq!(t.get("edition"), Some("yr"));
    }
}
