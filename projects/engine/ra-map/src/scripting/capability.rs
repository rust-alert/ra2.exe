//! 地图剧本能力缺口诊断。

use super::MapScripting;
use crate::MapInfo;

/// 一条地图域能力缺口。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MapCapabilityGap {
    /// 机器可读码（如 `map.action.42 unsupported`）。
    pub code: String,
    /// 人类可读说明。
    pub message: String,
}

/// 竖切已接线的 `[Actions]` 动作类型码（与 `ra-engine` 触发执行表对齐）。
///
/// | 码 | 含义 |
/// |----|------|
/// | 0 | None 无操作 |
/// | 1 | Win 胜利 |
/// | 2 | Lose 失败 |
/// | 4 | Create Team 创建小队 |
/// | 5 | Destroy Team 销毁小队 |
/// | 6 | All to Hunt 全员追击 |
/// | 7 | Reinforcement 增援小队 |
/// | 8 | Drop Zone Flare（无呈现 no-op） |
/// | 10 | Play Movie（无呈现 no-op） |
/// | 11 | Text Trigger（无呈现 no-op） |
/// | 12 | Destroy Trigger 销毁触发器 |
/// | 14 | Change House 改属阵营 |
/// | 15 | Allow Win 允许胜利 |
/// | 19 | Play Sound（无音频 no-op） |
/// | 20 | Play Theme（无音频 no-op） |
/// | 21 | Play Speech（无音频 no-op） |
/// | 22 | Force Trigger 强制触发 |
/// | 23 | Timer Start 恢复计时 |
/// | 24 | Timer Stop 暂停计时 |
/// | 25 | Timer Extend 延长计时 |
/// | 26 | Timer Shorten 缩短计时 |
/// | 27 | Timer Set 设置计时器 |
/// | 32 | Destroy Attached Objects 摧毁绑定对象 |
/// | 36 | All Change House 全员改属 |
/// | 37 | Make Ally 结盟 |
/// | 38 | Make Enemy 解盟 |
/// | 53 | Enable Trigger 启用触发器 |
/// | 54 | Disable Trigger 禁用触发器 |
/// | 70 | Destroy Tag 摧毁指定 Tag |
/// | 74 | AI triggers begin 启用 AITrigger |
/// | 75 | AI triggers stop 停用 AITrigger |
/// | 80 | Reinforcement（航点）增援小队 |
/// | 98 | Play Sound Effect（无音频 no-op） |
/// | 103 | Timer Text（无 UI no-op） |
/// | 119 | Destroy all of 摧毁阵营全部 |
/// | 120 | Destroy all Buildings of 摧毁阵营建筑 |
/// | 121 | Destroy all Land Units of 摧毁阵营陆上单位 |
const SUPPORTED_ACTION_KINDS: &[i32] = &[
    0,   // None
    1,   // Win
    2,   // Lose
    4,   // Create Team
    5,   // Destroy Team
    6,   // All to Hunt
    7,   // Reinforcement
    8,   // Drop Zone Flare (no-op)
    10,  // Play Movie (no-op)
    11,  // Text Trigger (no-op)
    12,  // Destroy Trigger
    14,  // Change House
    15,  // Allow Win
    19,  // Play Sound (no-op)
    20,  // Play Theme (no-op)
    21,  // Play Speech (no-op)
    22,  // Force Trigger
    23,  // Timer Start
    24,  // Timer Stop
    25,  // Timer Extend
    26,  // Timer Shorten
    27,  // Timer Set
    32,  // Destroy Attached Objects
    36,  // All Change House
    37,  // Make Ally
    38,  // Make Enemy
    53,  // Enable Trigger
    54,  // Disable Trigger
    70,  // Destroy Tag
    74,  // AI triggers begin
    75,  // AI triggers stop
    80,  // Reinforcement at waypoint
    98,  // Play Sound Effect (no-op)
    103, // Timer Text (no-op)
    119, // Destroy all of
    120, // Destroy all Buildings of
    121, // Destroy all Land Units of
];

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
            if SUPPORTED_ACTION_KINDS.contains(&cmd.kind) {
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
