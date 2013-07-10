//! 测试状态旁路：与 `ra-desktop` test-harness 写出的键值文件对齐。

use std::path::Path;

/// `RA2_TEST_STATUS_PATH` 文件内容。
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct TestStatus {
    pub tick: u64,
    pub hash: u64,
    pub outcome: String,
    pub paused: bool,
    pub selected: Vec<usize>,
    pub entities: usize,
}

impl TestStatus {
    pub fn parse(text: &str) -> Result<Self, String> {
        let mut status = Self::default();
        status.outcome = "none".into();
        for raw in text.lines() {
            let line = raw.trim();
            if line.is_empty() {
                continue;
            }
            let Some((key, value)) = line.split_once('=') else {
                return Err(format!("状态行缺少 '=': {line}"));
            };
            match key {
                "tick" => {
                    status.tick = value
                        .parse()
                        .map_err(|_| format!("无效 tick: {value}"))?;
                }
                "hash" => {
                    let v = value.strip_prefix("0x").unwrap_or(value);
                    status.hash =
                        u64::from_str_radix(v, 16).map_err(|_| format!("无效 hash: {value}"))?;
                }
                "outcome" => status.outcome = value.to_string(),
                "paused" => {
                    status.paused = match value {
                        "true" | "1" => true,
                        "false" | "0" => false,
                        other => return Err(format!("无效 paused: {other}")),
                    };
                }
                "selected" => {
                    status.selected = if value.is_empty() {
                        Vec::new()
                    } else {
                        value
                            .split(',')
                            .map(|s| {
                                s.parse::<usize>()
                                    .map_err(|_| format!("无效 selected: {value}"))
                            })
                            .collect::<Result<Vec<_>, _>>()?
                    };
                }
                "entities" => {
                    status.entities = value
                        .parse()
                        .map_err(|_| format!("无效 entities: {value}"))?;
                }
                _ => {}
            }
        }
        Ok(status)
    }

    pub fn read_file(path: &Path) -> Result<Self, String> {
        let text = std::fs::read_to_string(path).map_err(|e| e.to_string())?;
        Self::parse(&text)
    }

    /// 解释计划里的粗略期望串（当前支持 `tick>=N` / `outcome!=none` / `paused=true`）。
    pub fn matches_expect(&self, expect: &str) -> bool {
        let expect = expect.trim();
        if let Some(n) = expect.strip_prefix("tick>=") {
            return n.parse::<u64>().ok().is_some_and(|min| self.tick >= min);
        }
        if expect == "outcome!=none" {
            return self.outcome != "none" && !self.outcome.is_empty();
        }
        if expect == "paused=true" {
            return self.paused;
        }
        if expect == "paused=false" {
            return !self.paused;
        }
        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_sidecar_roundtrip_shape() {
        let text = "\
tick=12
hash=0xabc
outcome=victory:Americans
paused=true
selected=0,2
entities=2
";
        let s = TestStatus::parse(text).unwrap();
        assert_eq!(s.tick, 12);
        assert_eq!(s.hash, 0xabc);
        assert_eq!(s.outcome, "victory:Americans");
        assert!(s.paused);
        assert_eq!(s.selected, vec![0, 2]);
        assert_eq!(s.entities, 2);
        assert!(s.matches_expect("tick>=1"));
        assert!(s.matches_expect("outcome!=none"));
        assert!(s.matches_expect("paused=true"));
    }
}
