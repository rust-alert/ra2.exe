//! 地图剧本能力缺口诊断。

use super::MapScripting;
use crate::MapInfo;

/// 一条地图域能力缺口。
#[derive(Debug, Clone, PartialEq, Eq)]
#[doc(hidden)]
pub struct MapCapabilityGap {
    /// 机器可读码（如 `map.action.42 unsupported` / `map.action.99 stub`）。
    pub code: String,
    /// 人类可读说明。
    pub message: String,
}

/// 根据地图剧本数据生成能力缺口（不静默半可玩）。
pub fn map_scripting_capability_gaps(map: &MapInfo) -> Vec<MapCapabilityGap> {
    gaps_from_scripting(&map.scripting)
}

#[doc(hidden)]
pub fn gaps_from_scripting(scripting: &MapScripting) -> Vec<MapCapabilityGap> {
    let mut out = Vec::new();
    for name in &scripting.unknown_sections {
        out.push(MapCapabilityGap {
            code: format!("map.section.{name} unsupported"),
            message: format!("地图节 [{name}] 当前引擎未建模"),
        });
    }
    let mut seen_actions = Vec::new();
    for action in &scripting.actions {
        for cmd in &action.commands {
            if seen_actions.contains(&cmd.kind) {
                continue;
            }
            seen_actions.push(cmd.kind);
            let code = cmd.kind.code();
            if !cmd.kind.is_supported() {
                out.push(MapCapabilityGap {
                    code: format!("map.action.{code} unsupported"),
                    message: format!("触发动作码 {code} 当前引擎未执行"),
                });
            } else if cmd.kind.is_presentation_stub() {
                out.push(MapCapabilityGap {
                    code: format!("map.action.{code} stub"),
                    message: format!("触发动作码 {code} 仅为呈现占位，执行不改变玩法状态"),
                });
            }
        }
    }
    // AITriggerTypes 已由引擎 `tick_ai_triggers` 最小执行，不再报告为开局阻塞缺口。
    out
}

/// 是否为战役硬拒的动作缺口（仅 `map.action.* unsupported`）。
pub fn is_campaign_blocking_action_gap(code: &str) -> bool {
    code.starts_with("map.action.") && code.ends_with(" unsupported")
}

/// 战役开局：存在未接线动作时返回拒绝说明（呈现 stub 只 WARN，不拒开局）。
pub fn campaign_blocking_capability_message(map: &MapInfo) -> Option<String> {
    let reports = map_scripting_capability_gaps(map);
    let blocking: Vec<&MapCapabilityGap> = reports.iter().filter(|r| is_campaign_blocking_action_gap(&r.code)).collect();
    if blocking.is_empty() {
        return None;
    }
    let summary = blocking.iter().map(|r| r.code.as_str()).collect::<Vec<_>>().join(", ");
    Some(format!("战役地图含未实现剧本能力，拒绝静默开局: {summary}"))
}
