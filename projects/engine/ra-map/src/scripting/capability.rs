//! 地图剧本能力缺口诊断。

use super::{MapActionKind, MapScripting};
use crate::MapInfo;

/// 一条地图域能力缺口。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MapCapabilityGap {
    /// 机器可读码（如 `map.action.42 unsupported`）。
    pub code: String,
    /// 人类可读说明。
    pub message: String,
}

/// 根据地图剧本数据生成能力缺口（不静默半可玩）。
pub fn map_scripting_capability_gaps(map: &MapInfo) -> Vec<MapCapabilityGap> {
    gaps_from_scripting(&map.scripting)
}

fn gaps_from_scripting(scripting: &MapScripting) -> Vec<MapCapabilityGap> {
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
            if MapActionKind::is_supported(cmd.kind) {
                continue;
            }
            if seen_actions.contains(&cmd.kind) {
                continue;
            }
            seen_actions.push(cmd.kind);
            out.push(MapCapabilityGap {
                code: format!("map.action.{} unsupported", cmd.kind),
                message: format!("触发动作码 {} 当前引擎未执行", cmd.kind),
            });
        }
    }
    // AITriggerTypes 已由引擎 `tick_ai_triggers` 最小执行，不再报告为开局阻塞缺口。
    out
}

/// 战役开局：存在未接线动作时返回拒绝说明。
pub fn campaign_blocking_capability_message(map: &MapInfo) -> Option<String> {
    let reports = map_scripting_capability_gaps(map);
    let blocking: Vec<&MapCapabilityGap> = reports
        .iter()
        .filter(|r| r.code.starts_with("map.action."))
        .collect();
    if blocking.is_empty() {
        return None;
    }
    let summary = blocking
        .iter()
        .map(|r| r.code.as_str())
        .collect::<Vec<_>>()
        .join(", ");
    Some(format!("战役地图含未实现剧本能力，拒绝静默开局: {summary}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn supported_action_kinds_cover_enum_table() {
        assert!(MapActionKind::is_supported(MapActionKind::Win.as_i32()));
        assert!(MapActionKind::is_supported(MapActionKind::DestroyAllLandUnitsOf.as_i32()));
        assert!(!MapActionKind::is_supported(99));
        assert_eq!(MapActionKind::SUPPORTED.len(), 43);
    }
}
