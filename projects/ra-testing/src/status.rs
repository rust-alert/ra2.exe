//! 测试状态旁路：与 `ra-desktop` test-harness 写出的键值文件对齐。

use std::path::Path;

/// `RA2_TEST_STATUS_PATH` 文件内容。
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct TestStatus {
    /// 仿真 tick。
    pub tick: u64,
    /// 状态摘要（十六进制解析后的值）。
    pub hash: u64,
    /// 胜负文案，未结束时为 `none`。
    pub outcome: String,
    /// 会话是否暂停。
    pub paused: bool,
    /// 当前选中实体下标。
    pub selected: Vec<usize>,
    /// 实体总数。
    pub entities: usize,
    /// 本地玩家资金。
    pub funds: i32,
    /// 本地玩家供电。
    pub power_output: i32,
    /// 本地玩家耗电。
    pub power_drain: i32,
    /// 是否低电。
    pub low_power: bool,
    /// 生产队列摘要，如 `E1:12`；无则为 `none`。
    pub queue: String,
    /// 最近拒绝原因调试名，无则为 `none`。
    pub last_reject: String,
}

impl TestStatus {
    /// 解析旁路文本。
    pub fn parse(text: &str) -> Result<Self, String> {
        let mut status = Self::default();
        status.outcome = "none".into();
        status.queue = "none".into();
        status.last_reject = "none".into();
        for raw in text.lines() {
            let line = raw.trim();
            if line.is_empty() {
                continue;
            }
            let Some((key, value)) = line.split_once('=')
            else {
                return Err(format!("状态行缺少 '=': {line}"));
            };
            match key {
                "tick" => {
                    status.tick = value.parse().map_err(|_| format!("无效 tick: {value}"))?;
                }
                "hash" => {
                    let v = value.strip_prefix("0x").unwrap_or(value);
                    status.hash = u64::from_str_radix(v, 16).map_err(|_| format!("无效 hash: {value}"))?;
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
                    }
                    else {
                        value
                            .split(',')
                            .map(|s| s.parse::<usize>().map_err(|_| format!("无效 selected: {value}")))
                            .collect::<Result<Vec<_>, _>>()?
                    };
                }
                "entities" => {
                    status.entities = value.parse().map_err(|_| format!("无效 entities: {value}"))?;
                }
                "funds" => {
                    status.funds = value.parse().map_err(|_| format!("无效 funds: {value}"))?;
                }
                "power_output" => {
                    status.power_output = value.parse().map_err(|_| format!("无效 power_output: {value}"))?;
                }
                "power_drain" => {
                    status.power_drain = value.parse().map_err(|_| format!("无效 power_drain: {value}"))?;
                }
                "low_power" => {
                    status.low_power = match value {
                        "true" | "1" => true,
                        "false" | "0" => false,
                        other => return Err(format!("无效 low_power: {other}")),
                    };
                }
                "queue" => status.queue = value.to_string(),
                "last_reject" => status.last_reject = value.to_string(),
                _ => {}
            }
        }
        Ok(status)
    }

    /// 从文件读取并解析。
    pub fn read_file(path: &Path) -> Result<Self, String> {
        let text = std::fs::read_to_string(path).map_err(|e| e.to_string())?;
        Self::parse(&text)
    }

    /// 解释计划里的粗略期望串（当前支持 `tick>=N` / `outcome!=none` / `paused=true|false` / `funds>=N`）。
    pub fn matches_expect(&self, expect: &str) -> bool {
        let expect = expect.trim();
        if let Some(n) = expect.strip_prefix("tick>=") {
            return n.parse::<u64>().ok().is_some_and(|min| self.tick >= min);
        }
        if let Some(n) = expect.strip_prefix("funds>=") {
            return n.parse::<i32>().ok().is_some_and(|min| self.funds >= min);
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
        if expect == "queue!=none" {
            return self.queue != "none" && !self.queue.is_empty();
        }
        false
    }
}
