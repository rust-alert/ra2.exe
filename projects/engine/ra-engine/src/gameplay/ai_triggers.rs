//! 地图 `[AITriggerTypes]` 最小执行：按条件与冷却排队 Create Team。

use std::collections::HashMap;

use ra_map::MapEntityKind;
use ra_types::{AiTriggerConditionKind, AiTriggerId, HouseId, PreparedAiTrigger};

use crate::{
    gameplay::{houses_are_allied, is_ambient_house},
    state::{
        BattleState,
        components::{Health, Identity, Owner},
    },
};

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
    /// 会话难度标签（`Easy` / `Normal` / `Hard`）；影响 `enabled_*` 门控。
    difficulty: String,
    /// 是否遭遇战会话；为真时要求 `for_skirmish`。
    skirmish: bool,
}

impl AiTriggerRuntime {
    /// 从地图播种：有条目则默认启用。
    pub fn from_map(has_triggers: bool) -> Self {
        Self {
            enabled: has_triggers,
            cooldowns: HashMap::new(),
            disabled_houses: Vec::new(),
            difficulty: "Normal".into(),
            skirmish: false,
        }
    }

    /// 由 `BattleSession` 每 tick 同步难度与遭遇战标记。
    pub fn sync_session_hints(&mut self, difficulty: &str, skirmish: bool) {
        self.difficulty = difficulty.to_string();
        self.skirmish = skirmish;
    }
}

/// 每个逻辑 tick：冷却归零且条件成立的 AITrigger 将 TeamType 排入 `pending_team_spawns`。
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
        let owner_house_key = resolve_owner_house_key(world, &at);
        if let Some(ref house_key) = owner_house_key {
            if let Some(house_id) = world.definitions.houses.get(house_key).map(|h| h.id) {
                if world.ai_trigger_runtime.disabled_houses.contains(&house_id) {
                    continue;
                }
            }
            world.ensure_house(house_key);
            // 战役：未「Production Begins」的房主不由 AITrigger 刷队。
            if !world.house_production_begun(house_key) {
                continue;
            }
            let tech = world
                .players
                .iter()
                .find(|p| p.house.as_ref().eq_ignore_ascii_case(house_key))
                .map(|p| p.tech_level)
                .unwrap_or(0);
            if tech < at.tech_level {
                continue;
            }
        }

        let cooling = world.ai_trigger_runtime.cooldowns.get(&at.id).copied().unwrap_or(0);
        if cooling > 0 {
            continue;
        }

        if world.ai_trigger_runtime.skirmish && !at.for_skirmish {
            continue;
        }
        if !ai_trigger_enabled_for_difficulty(&world.ai_trigger_runtime.difficulty, &at) {
            continue;
        }

        let Some(team) = world.prepared.team_types.iter().find(|t| t.id == at.team).cloned()
        else {
            continue;
        };
        // 无 OwnerHouse 时，用 TeamType.House 约束战役生产开关。
        if owner_house_key.is_none() {
            if let Some(house_key) = world.definitions.houses.get_by_id(team.house).map(|h| h.type_key.as_str().to_string()) {
                world.ensure_house(&house_key);
                if !world.house_production_begun(&house_key) {
                    continue;
                }
            }
        }

        let owner_for_cond = owner_house_key
            .clone()
            .or_else(|| world.definitions.houses.get_by_id(team.house).map(|h| h.type_key.as_str().to_string()));
        if let Some(ref owner) = owner_for_cond {
            if !ai_trigger_condition_holds(world, owner, &at) {
                continue;
            }
        }
        else if !matches!(at.condition, AiTriggerConditionKind::Always) {
            continue;
        }

        // `Max=`：已有同 TeamType 活跃小队达到上限则跳过本拍。
        if team.max > 0 {
            let active = world.script_team_runtime.count_active_of_type(team.id);
            if active >= team.max as usize {
                world.ai_trigger_runtime.cooldowns.insert(at.id, AI_TRIGGER_COOLDOWN_TICKS);
                continue;
            }
        }
        if world.trigger_runtime.pending_team_spawns.contains(&team.id) {
            continue;
        }
        world.trigger_runtime.pending_team_spawns.push(team.id);
        world.ai_trigger_runtime.cooldowns.insert(at.id, AI_TRIGGER_COOLDOWN_TICKS);
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

fn resolve_owner_house_key(world: &BattleState, at: &PreparedAiTrigger) -> Option<String> {
    at.owner_house.and_then(|id| world.definitions.houses.get_by_id(id).map(|h| h.type_key.as_str().to_string()))
}

fn ai_trigger_enabled_for_difficulty(difficulty: &str, at: &PreparedAiTrigger) -> bool {
    if difficulty.eq_ignore_ascii_case("Easy") {
        at.enabled_easy
    }
    else if difficulty.eq_ignore_ascii_case("Hard") {
        at.enabled_hard
    }
    else {
        at.enabled_normal
    }
}

fn ai_trigger_condition_holds(world: &BattleState, owner_house: &str, at: &PreparedAiTrigger) -> bool {
    match at.condition {
        AiTriggerConditionKind::Always => true,
        AiTriggerConditionKind::Unsupported(_) => false,
        AiTriggerConditionKind::EnemyOwns => {
            let n = count_type_owned_by(world, &at.condition_object, |h| {
                !h.eq_ignore_ascii_case(owner_house) && !houses_are_allied(world, owner_house, h) && !is_ambient_house(h)
            });
            at.compare_op.compare(n, at.compare_amount)
        }
        AiTriggerConditionKind::OwnOwns => {
            let n = count_type_owned_by(world, &at.condition_object, |h| h.eq_ignore_ascii_case(owner_house));
            at.compare_op.compare(n, at.compare_amount)
        }
        AiTriggerConditionKind::NeutralOwns => {
            let n = count_type_owned_by(world, &at.condition_object, is_ambient_house);
            at.compare_op.compare(n, at.compare_amount)
        }
        AiTriggerConditionKind::EnemyYellowPower => any_enemy_player(world, owner_house, |p| p.low_power()),
        AiTriggerConditionKind::EnemyRedPower => any_enemy_player(world, owner_house, |p| {
            p.power_drain > 0 && p.effective_power_output() == 0
        }),
        AiTriggerConditionKind::EnemyCredits => {
            let credits = max_enemy_funds(world, owner_house);
            at.compare_op.compare(credits, at.compare_amount)
        }
    }
}

fn count_type_owned_by<F>(world: &BattleState, type_key: &ra_types::TechnoName, house_ok: F) -> i32
where
    F: Fn(&str) -> bool,
{
    if type_key.is_empty() {
        return 0;
    }
    let needle = type_key.as_str();
    let mut n = 0i32;
    for e in &world.entities {
        let id = e.id;
        if world.ecs_get::<Health>(id).map(|h| h.dead).unwrap_or(true) {
            continue;
        }
        let Some(owner) = world.ecs_get::<Owner>(id)
        else {
            continue;
        };
        if !house_ok(owner.house.as_ref()) {
            continue;
        }
        let Some(identity) = world.ecs_get::<Identity>(id)
        else {
            continue;
        };
        if identity.type_id.as_ref().eq_ignore_ascii_case(needle) {
            n = n.saturating_add(1);
            // 建筑按座计数即可；Foundation 展开不在 ECS Identity 重复。
            let _ = identity.kind == MapEntityKind::Structure;
        }
    }
    n
}

fn any_enemy_player<F>(world: &BattleState, owner_house: &str, pred: F) -> bool
where
    F: Fn(&crate::PlayerState) -> bool,
{
    world.players.iter().any(|p| {
        !p.house.as_ref().eq_ignore_ascii_case(owner_house)
            && !houses_are_allied(world, owner_house, p.house.as_ref())
            && !is_ambient_house(p.house.as_ref())
            && pred(p)
    })
}

fn max_enemy_funds(world: &BattleState, owner_house: &str) -> i32 {
    world
        .players
        .iter()
        .filter(|p| {
            !p.house.as_ref().eq_ignore_ascii_case(owner_house)
                && !houses_are_allied(world, owner_house, p.house.as_ref())
                && !is_ambient_house(p.house.as_ref())
        })
        .map(|p| p.funds)
        .max()
        .unwrap_or(0)
}

#[cfg(test)]
mod compare_op_tests {
    use ra_types::AiTriggerCompareOp;

    #[test]
    fn compare_ops_match_spec() {
        assert!(AiTriggerCompareOp::GreaterEqual.compare(1, 1));
        assert!(!AiTriggerCompareOp::Greater.compare(1, 1));
        assert!(AiTriggerCompareOp::Less.compare(0, 1));
    }
}
