//! 桌面启动配置（GUI 应用，不是命令行参数）。

use std::path::{Path, PathBuf};

#[derive(Debug, Clone)]
pub struct DesktopConfig {
    /// 含零售 MIX / INI 的目录。
    pub ra2_dir: PathBuf,
    /// 可选 `ra2` / `yr`；`None` 表示自动探测。
    pub edition: Option<String>,
}

impl Default for DesktopConfig {
    fn default() -> Self {
        Self {
            ra2_dir: PathBuf::from("."),
            edition: None,
        }
    }
}

impl DesktopConfig {
    pub fn load_or_default() -> Self {
        for candidate in [Path::new("config.toml"), Path::new("ra2.toml")] {
            if let Ok(text) = std::fs::read_to_string(candidate) {
                return Self::parse_toml_lite(&text);
            }
        }
        Self::default()
    }

    pub fn game_dir(&self) -> PathBuf {
        self.ra2_dir.clone()
    }

    /// 极简键值读取——够启动，不必上 CLI 栈。
    fn parse_toml_lite(text: &str) -> Self {
        let mut cfg = Self::default();
        for raw in text.lines() {
            let line = raw.split('#').next().unwrap_or("").trim();
            if line.is_empty() || line.starts_with('[') {
                continue;
            }
            let Some((k, v)) = line.split_once('=') else {
                continue;
            };
            let key = k.trim();
            let val = v.trim().trim_matches('"').trim_matches('\'');
            match key {
                "ra2_dir" | "game_dir" => cfg.ra2_dir = PathBuf::from(val),
                "edition" if !val.is_empty() => cfg.edition = Some(val.to_string()),
                _ => {}
            }
        }
        cfg
    }
}
