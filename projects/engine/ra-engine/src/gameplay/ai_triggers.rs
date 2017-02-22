//! 地图 `[AITriggerTypes]` 最小执行：按冷却排队 Create Team。

use std::collections::HashMap;

use ra_types::{AiTriggerId, HouseId};

use crate::state::BattleState;

/// AITrigger 默认冷却（逻辑 tick）。过短会刷队，过长像“不出兵”。
const AI_TRIGGER_COOLDOWN_TICKS: u32 = 90;

/// AITrigger 运行时。
#[derive(Debug, Clone, Default)]
pub struct AiTriggerRuntime {
    /// 全局开关（`AI triggers begin/stop` 可改；默认开）。
    pub enabled: bool,
    /// 各 AITrigger 稳定 id → 剩余冷却 tick。
    cooldowns: HashMap<AiTriggerId, u32>,
    /// 按 house 禁用（`AI triggers stop`）；缺省未列入则允许。
    disabled_houses: Vec<HouseId>,
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
    let triggers = world.prepared.ai_triggers.clone();
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
        if let Some(house_id) = at.owner_house {
            if world.ai_trigger_runtime.disabled_houses.contains(&house_id) {
                continue;
            }
            let Some(house_key) = world.definitions.houses.get_by_id(house_id).map(|h| h.type_key.as_str().to_string())
            else {
                continue;
            };
            world.ensure_house(&house_key);
            let tech = world
                .players
                .iter()
                .find(|p| p.house.as_ref().eq_ignore_ascii_case(&house_key))
                .map(|p| p.tech_level)
                .unwrap_or(0);
            if tech < at.tech_level {
                continue;
            }
        }
        let rem = world.ai_trigger_runtime.cooldowns.entry(at.id).or_insert(0);
        if *rem > 0 {
            continue;
        }
        let Some(team) = world.prepared.team_types.iter().find(|t| t.id == at.team)
        else {
            continue;
        };
        world.trigger_runtime.pending_team_spawns.push(team.id);
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
            let Some(house_id) = world.definitions.houses.get(h).map(|d| d.id)
            else {
                return;
            };
            if enabled {
                world.ai_trigger_runtime.disabled_houses.retain(|x| *x != house_id);
            }
            else if !world.ai_trigger_runtime.disabled_houses.contains(&house_id) {
                world.ai_trigger_runtime.disabled_houses.push(house_id);
            }
        }
    }
}
