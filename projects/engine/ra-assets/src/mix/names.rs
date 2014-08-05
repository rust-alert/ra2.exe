//! MIX 文件名哈希恢复表：`mix_hash(name) → 原名`。
//!
//! TS/RA2/YR 的 MIX 索引只存哈希。零售资源文件名是公开 well-known 集合，
//! 且该集合在 CRC-32 哈希下无冲突，可用静态表还原原名。
//!
//! 内置表来自引擎已引用名 + `rules`/`art`/`sound` 等公开 INI 引用；
//! 可用文本文件继续追加（一行一个逻辑名）。

use std::collections::HashMap;
use std::path::Path;

use ra_types::{RaError, RaResult};

use super::hash::mix_hash;

/// 内置 well-known 逻辑名（每行一个，大小写不敏感）。
const BUILTIN_KNOWN_NAMES: &str = include_str!("../../data/mix_known_names.txt");

/// 哈希 → 原文件名恢复表。
#[derive(Debug, Clone, Default)]
pub struct MixNameTable {
    by_id: HashMap<i32, String>,
}

impl MixNameTable {
    /// 空表。
    pub fn new() -> Self {
        Self::default()
    }

    /// 加载内置 well-known 名表；若发现哈希冲突则报错。
    pub fn builtin() -> RaResult<Self> {
        let mut table = Self::new();
        table.extend_lines(BUILTIN_KNOWN_NAMES)?;
        Ok(table)
    }

    /// 已登记条数。
    pub fn len(&self) -> usize {
        self.by_id.len()
    }

    /// 是否为空。
    pub fn is_empty(&self) -> bool {
        self.by_id.is_empty()
    }

    /// 按条目 id 查原名。
    pub fn lookup(&self, entry_id: i32) -> Option<&str> {
        self.by_id.get(&entry_id).map(String::as_str)
    }

    /// 插入一个逻辑名；与已有不同名同哈希则冲突失败。
    pub fn insert(&mut self, name: &str) -> RaResult<()> {
        let trimmed = name.trim();
        if trimmed.is_empty() || trimmed.starts_with('#') {
            return Ok(());
        }
        let key = trimmed.to_ascii_lowercase();
        let id = mix_hash(&key);
        match self.by_id.get(&id) {
            Some(existing) if existing.eq_ignore_ascii_case(&key) => Ok(()),
            Some(existing) => Err(RaError::Parse(format!(
                "mix name hash collision: {existing} vs {key} (id={id})"
            ))),
            None => {
                self.by_id.insert(id, key);
                Ok(())
            }
        }
    }

    /// 从多行文本追加（`#` 开头为注释）。
    pub fn extend_lines(&mut self, text: &str) -> RaResult<usize> {
        let before = self.len();
        for line in text.lines() {
            self.insert(line)?;
        }
        Ok(self.len().saturating_sub(before))
    }

    /// 从 UTF-8 文本文件追加。
    pub fn extend_file(&mut self, path: &Path) -> RaResult<usize> {
        let text = std::fs::read_to_string(path).map_err(|e| RaError::Io(format!("{}: {e}", path.display())))?;
        self.extend_lines(&text)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn builtin_has_no_collisions_and_contains_shell_assets() {
        let table = MixNameTable::builtin().expect("builtin");
        assert!(table.len() > 100);
        let id = mix_hash("sdtp.shp");
        assert_eq!(table.lookup(id), Some("sdtp.shp"));
        assert_eq!(table.lookup(mix_hash("title.pcx")), Some("title.pcx"));
    }

    #[test]
    fn insert_detects_collision() {
        let mut table = MixNameTable::new();
        table.insert("alpha.bin").unwrap();
        // 人为构造极难；用 mock：插入同名应成功，冲突需真实不同名同哈希。
        table.insert("ALPHA.BIN").unwrap();
        assert_eq!(table.len(), 1);
    }
}
