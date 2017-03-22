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
    /// 各 AITrigger 当前动态权重（缺省取 prepared `weight`）。
    current_weights: HashMap<AiTriggerId, u32>,
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
            current_weights: HashMap::new(),
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

    /// 当前动态权重（测试 / 诊断）；尚未抖动时回退 `start`。
    pub fn current_weight(&self, id: AiTriggerId, start: u32) -> u32 {
        self.current_weights.get(&id).copied().unwrap_or(start.max(1))
    }
}

/// 每个逻辑 tick：冷却归零且条件成立的 AITrigger 按房主加权抽选一支，将 TeamType 排入 `pending_team_spawns`。
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

    let mut eligible: Vec<(String, PreparedAiTrigger)> = Vec::new();
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
            let tech = world.players.iter().find(|p| p.house.as_ref().eq_ignore_ascii_case(house_key)).map(|p| p.tech_level).unwrap_or(0);
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
        let group_key = if let Some(ref hk) = owner_house_key {
            hk.clone()
        }
        else if let Some(house_key) = world.definitions.houses.get_by_id(team.house).map(|h| h.type_key.as_str().to_string()) {
            world.ensure_house(&house_key);
            if !world.house_production_begun(&house_key) {
                continue;
            }
            house_key
        }
        else {
            String::new()
        };

        let owner_for_cond = if group_key.is_empty() { None } else { Some(group_key.clone()) };
        if let Some(ref owner) = owner_for_cond {
            if !ai_trigger_condition_holds(world, owner, &at) {
                // 条件失败：当前权重向 `max_weight` 收敛，提高后续抽中率。
                nudge_weight_toward(world, &at, at.max_weight);
                continue;
            }
        }
        else if !matches!(at.condition, AiTriggerConditionKind::Always) {
            nudge_weight_toward(world, &at, at.max_weight);
            continue;
        }

        eligible.push((group_key, at));
    }

    // 同房主每 tick 至多抽选一支触发（动态权重）；避免条件全过时同拍刷多队。
    let mut groups: HashMap<String, Vec<PreparedAiTrigger>> = HashMap::new();
    for (key, at) in eligible {
        groups.entry(key).or_default().push(at);
    }
    let mut group_keys: Vec<String> = groups.keys().cloned().collect();
    group_keys.sort();
    for key in group_keys {
        let Some(cands) = groups.remove(&key)
        else {
            continue;
        };
        let Some(picked) = pick_weighted_trigger(world, &key, &cands)
        else {
            continue;
        };
        // 未抽中的候选向 `max_weight` 回升。
        for c in &cands {
            if c.id != picked.id {
                nudge_weight_toward(world, c, c.max_weight);
            }
        }
        enqueue_ai_trigger_teams(world, &picked);
        // 抽中并入队：向 `min_weight` 收敛，避免同一触发连刷。
        nudge_weight_toward(world, &picked, picked.min_weight);
    }
}

fn effective_weight(world: &BattleState, at: &PreparedAiTrigger) -> u32 {
    world.ai_trigger_runtime.current_weight(at.id, at.weight).max(1)
}

fn nudge_weight_toward(world: &mut BattleState, at: &PreparedAiTrigger, target: u32) {
    let cur = effective_weight(world, at);
    let lo = at.min_weight.max(1);
    let hi = at.max_weight.max(lo);
    let target = target.clamp(lo, hi);
    let next = ((u64::from(cur) + u64::from(target)) / 2) as u32;
    world.ai_trigger_runtime.current_weights.insert(at.id, next.max(1));
}

fn pick_weighted_trigger(world: &BattleState, group_key: &str, cands: &[PreparedAiTrigger]) -> Option<PreparedAiTrigger> {
    if cands.is_empty() {
        return None;
    }
    if cands.len() == 1 {
        return cands.first().cloned();
    }
    let total: u64 = cands.iter().map(|c| u64::from(effective_weight(world, c))).sum();
    if total == 0 {
        return cands.first().cloned();
    }
    let mut roll = deterministic_roll(world.match_seed, world.tick, group_key) % total;
    for c in cands {
        let w = u64::from(effective_weight(world, c));
        if roll < w {
            return Some(c.clone());
        }
        roll -= w;
    }
    cands.last().cloned()
}

fn deterministic_roll(seed: u64, tick: u64, group_key: &str) -> u64 {
    let mut h = seed ^ tick.wrapping_mul(0x9E37_79B9_7F4A_7C15);
    for b in group_key.as_bytes() {
        h = h.wrapping_mul(1099511628211).wrapping_add(u64::from(*b));
    }
    h
}

fn enqueue_ai_trigger_teams(world: &mut BattleState, at: &PreparedAiTrigger) {
    let mut teams = vec![at.team];
    if let Some(team2) = at.team2 {
        if team2 != at.team {
            teams.push(team2);
        }
    }
    for team_id in teams {
        if world.trigger_runtime.pending_team_spawns.contains(&team_id) {
            continue;
        }
        if let Some(team) = world.prepared.team_types.iter().find(|t| t.id == team_id) {
            if team.max > 0 {
                let active = world.script_team_runtime.count_active_of_type(team.id);
                if active >= team.max as usize {
                    continue;
                }
            }
        }
        world.trigger_runtime.pending_team_spawns.push(team_id);
    }
    world.ai_trigger_runtime.cooldowns.insert(at.id, AI_TRIGGER_COOLDOWN_TICKS);
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
        AiTriggerConditionKind::EnemyRedPower => any_enemy_player(world, owner_house, |p| p.power_drain > 0 && p.effective_power_output() == 0),
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
