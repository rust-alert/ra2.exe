//! 地图 `[AITriggerTypes]` 最小执行：按冷却排队 Create Team。

use std::collections::HashMap;

use crate::state::BattleState;

/// AITrigger 默认冷却（逻辑 tick）。过短会刷队，过长像“不出兵”。
const AI_TRIGGER_COOLDOWN_TICKS: u32 = 90;

/// AITrigger 运行时。
#[derive(Debug, Clone, Default)]
pub struct AiTriggerRuntime {
    /// 全局开关（`AI triggers begin/stop` 可改；默认开）。
    pub enabled: bool,
    /// 各 AITrigger id → 剩余冷却 tick。
    cooldowns: HashMap<String, u32>,
    /// 按 house 禁用（`AI triggers stop`）；缺省未列入则允许。
    disabled_houses: Vec<String>,
}

impl AiTriggerRuntime {
    /// 从地图播种：有条目则默认启用。
    pub fn from_map(has_triggers: bool) -> Self {
        Self { enabled: has_triggers, cooldowns: HashMap::new(), disabled_houses: Vec::new() }
    }
}

/// 每个逻辑 tick：冷却归零的 AITrigger 将 TeamType 排入 `pending_team_spawns`。
pub fn tick_ai_triggers(world: &mut BattleState) {
    if !world.ai_trigger_runtime.enabled {
        return;
    }
    let triggers = world.map.scripting.ai_triggers.clone();
    if triggers.is_empty() {
        return;
    }

    // 先推进冷却。
    for rem in world.ai_trigger_runtime.cooldowns.values_mut() {
        if *rem > 0 {
            *rem = rem.saturating_sub(1);
        }
    }

    for at in triggers {
        if at.team.trim().is_empty() {
            continue;
        }
        if !at.owner_house.is_empty() && world.ai_trigger_runtime.disabled_houses.iter().any(|h| h.eq_ignore_ascii_case(&at.owner_house)) {
            continue;
        }
        if !at.owner_house.is_empty() {
            world.ensure_house(&at.owner_house);
            let tech = world.players.iter().find(|p| p.house.as_ref().eq_ignore_ascii_case(&at.owner_house)).map(|p| p.tech_level).unwrap_or(0);
            if tech < at.tech_level {
                continue;
            }
        }
        let rem = world.ai_trigger_runtime.cooldowns.entry(at.id.clone()).or_insert(0);
        if *rem > 0 {
            continue;
        }
        world.trigger_runtime.pending_team_spawns.push(at.team.clone());
        *rem = AI_TRIGGER_COOLDOWN_TICKS;
    }
}

/// 启用或禁用某 house 的 AITrigger（空 house = 全局开关）。
pub fn set_ai_triggers_for_house(world: &mut BattleState, house: Option<&str>, enabled: bool) {
    match house {
        None | Some("") => {
            world.ai_trigger_runtime.enabled = enabled;
        }
        Some(h) => {
            if enabled {
                world.ai_trigger_runtime.disabled_houses.retain(|x| !x.eq_ignore_ascii_case(h));
            }
            else if !world.ai_trigger_runtime.disabled_houses.iter().any(|x| x.eq_ignore_ascii_case(h)) {
                world.ai_trigger_runtime.disabled_houses.push(h.to_string());
            }
        }
    }
}
