//! 基础 AI：只经 `GameCommand` 下发，不直接改写世界。
//!
//! 基建下一座建筑**只**从冻结 [`ra_types::AiControls`]（rules `[AI]` / `[IQ]`）候选表选取，
//! 不按外部类型名或写死步骤猜。单位量产仍由定义 + Owner 过滤选出。
//!
//! 有 `[AITriggerTypes]` 覆盖的房主：经济类可由本模块补齐，作战部队交给
//! `tick_ai_triggers` → Create Team / ScriptTypes，避免与启发式工厂量产叠刷。

use crate::{
    BattleState, GameCommand,
    gameplay::{TechTreePlayer, deploy_into_type, factory_matches_category, is_construction_yard, is_type_eligible_id, living_structure_keys},
    state::components::{AttackState, CombatStats, Health, Identity, Owner, ProductionQueue, Transform},
};
use ra_map::MapEntityKind;
use ra_types::{AiBuildCategory, AiControls, PlayerId, ProductionCategory, TechnoCategory, TypeId};

/// 启发式量产：同房主存活机动作战单位上限（达到后停刷）。
const HEURISTIC_ARMY_CAP: usize = 8;
/// 启发式量产：首批单位之后的下单间隔（逻辑 tick）。
const HEURISTIC_PRODUCE_PERIOD: u64 = 45;

/// 该房主的作战部队是否应由 `[AITriggerTypes]` 驱动（而非工厂启发式量产）。
pub fn house_army_driven_by_ai_triggers(world: &BattleState, house: &str) -> bool {
    if world.prepared.ai_triggers.is_empty() {
        return false;
    }
    for at in &world.prepared.ai_triggers {
        if let Some(house_id) = at.owner_house {
            if world.definitions.houses.get_by_id(house_id).is_some_and(|h| h.type_key.as_str().eq_ignore_ascii_case(house)) {
                return true;
            }
            continue;
        }
        if let Some(team) = world.prepared.team_types.iter().find(|t| t.id == at.team) {
            match team.house {
                None => return true,
                Some(team_house) => {
                    if world.definitions.houses.get_by_id(team_house).is_some_and(|h| h.type_key.as_str().eq_ignore_ascii_case(house)) {
                        return true;
                    }
                }
            }
        }
    }
    false
}

/// 无 AITrigger 覆盖时：是否允许本 tick 启发式工厂量产（首批放行，其后节流并封顶）。
/// 另要求房主 IQ ≥ `[IQ] Production`。
pub fn heuristic_should_produce_army(world: &BattleState, house: &str) -> bool {
    if !iq_allows(world, house, world.definitions.ai_controls.iq_production) {
        return false;
    }
    let army = count_mobile_combatants(world, house);
    if army >= HEURISTIC_ARMY_CAP {
        return false;
    }
    if army == 0 {
        return true;
    }
    world.tick % HEURISTIC_PRODUCE_PERIOD == 0
}

fn count_mobile_combatants(world: &BattleState, house: &str) -> usize {
    world
        .entities
        .iter()
        .filter(|e| {
            let id = e.id;
            if world.ecs_get::<Health>(id).map(|h| h.dead).unwrap_or(true) {
                return false;
            }
            if !world.ecs_get::<Owner>(id).map(|o| crate::gameplay::house_id_of(&world.definitions, house) == Some(o.house)).unwrap_or(false) {
                return false;
            }
            let Some(identity) = world.ecs_get::<Identity>(id)
            else {
                return false;
            };
            matches!(identity.kind, MapEntityKind::Unit | MapEntityKind::Infantry | MapEntityKind::Aircraft)
                && world.ecs_get::<CombatStats>(id).map(|s| s.attack_damage > 0).unwrap_or(false)
        })
        .count()
}

/// 地图氛围房主（平民装饰 / 多人被动），不参与遭遇战 AI，也不计入胜负作战力量。
///
/// 优先查冻结 `HouseDefinition.role`；表中缺失时才回落原版环境房屋名（装载缺口兜底）。
pub fn is_ambient_house(defs: &ra_types::RuntimeDefinitions, house: &str) -> bool {
    if let Some(h) = defs.houses.get(house) {
        return h.role.is_ambient();
    }
    ra_types::HouseRole::from_stock_ambient_name(house).is_some()
}

/// 两 house 是否同盟（同名，或任一方 `PlayerState.allies` 列出对方）。
pub fn houses_are_allied(world: &BattleState, a: &str, b: &str) -> bool {
    if a.eq_ignore_ascii_case(b) {
        return true;
    }
    let a_lists_b = world
        .players
        .iter()
        .find(|p| p.house.as_ref().eq_ignore_ascii_case(a))
        .map(|p| p.allies.iter().any(|x| x.eq_ignore_ascii_case(b)))
        .unwrap_or(false);
    if a_lists_b {
        return true;
    }
    world
        .players
        .iter()
        .find(|p| p.house.as_ref().eq_ignore_ascii_case(b))
        .map(|p| p.allies.iter().any(|x| x.eq_ignore_ascii_case(a)))
        .unwrap_or(false)
}

/// 为本阵营未部署的可部署单位生成 `Deploy`（已有建造场则跳过）。
pub fn deploy_mcv_commands(world: &BattleState, house: &str) -> Vec<GameCommand> {
    if house_has_yard(world, house) {
        return Vec::new();
    }
    let mut out = Vec::new();
    for e in world.entities.iter() {
        let id = e.id;
        if world.ecs_get::<Health>(id).map(|h| h.dead).unwrap_or(true) {
            continue;
        }
        if !world.ecs_get::<Owner>(id).map(|o| crate::gameplay::house_id_of(&world.definitions, house) == Some(o.house)).unwrap_or(false) {
            continue;
        }
        let Some(identity) = world.ecs_get::<Identity>(id)
        else {
            continue;
        };
        if deploy_into_type(&world.definitions, identity.type_id).is_none() {
            continue;
        }
        if identity.kind != MapEntityKind::Unit {
            continue;
        }
        out.push(GameCommand::Deploy { entity: id });
    }
    out
}

/// 有建造场时推进下一座基建（每 tick 至多一座）。
///
/// 若地图有该房主的 `[Base]` 计划且仍有未完成节点：只按节点类型生产 / 落位。
/// 节点耗尽后回落 [`AiControls`]；有节点的地图另受 `BaseSizeAdd` 总建筑封顶。
/// 无 `[Base]` 时行为与仅读 [`AiControls`] 时一致（不套用 `BaseSizeAdd` 封顶）。
pub fn next_structure_commands(world: &BattleState, house: &str, player: PlayerId, include_army_factories: bool) -> Vec<GameCommand> {
    if !house_has_yard(world, house) {
        return Vec::new();
    }
    if let Some(ready) = world.house_ready_building(house) {
        let Some(structure) = world.definitions.structures.get_by_id(ready)
        else {
            return Vec::new();
        };
        return place_structure(world, house, player, structure);
    }
    if let Some(node) = next_incomplete_base_node(world, house) {
        let Some(structure) = world.definitions.structures.get_by_id(node.type_id)
        else {
            return Vec::new();
        };
        if structure.construction_yard {
            return Vec::new();
        }
        let living = living_structure_keys(world, house);
        let Some(p) = world.players.iter().find(|p| p.house.eq_ignore_ascii_case(house))
        else {
            return Vec::new();
        };
        let tech = TechTreePlayer::from_player(p);
        if !is_type_eligible_id(&world.definitions, tech, &living, node.type_id) {
            return Vec::new();
        }
        return queue_structure(world, house, player, structure);
    }
    if base_plan_for_house(world, house).is_some() && !base_size_add_allows_freeform(world, house) {
        return Vec::new();
    }
    let controls = &world.definitions.ai_controls;
    let iq = house_iq(world, house);
    let can_expand = iq >= controls.iq_production;

    for kind in ai_category_order(include_army_factories) {
        if kind.requires_production_iq() && !can_expand {
            continue;
        }
        if !category_needed(world, house, controls, kind) {
            continue;
        }
        let Some(structure) = pick_from_candidates(world, house, kind.candidates(controls))
        else {
            // 供电表为空且仍需要电：卡住，避免跳去造别的。
            if matches!(kind, AiCategory::Power) {
                return Vec::new();
            }
            continue;
        };
        return queue_structure(world, house, player, structure);
    }
    Vec::new()
}

/// rules `[AI]` 建造类别（顺序固定，候选来自表）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum AiCategory {
    Power,
    Refinery,
    Barracks,
    Weapons,
    Radar,
    Tech,
    NavalYard,
    Helipad,
    Defense,
    Aa,
}

impl AiCategory {
    fn requires_production_iq(self) -> bool {
        !matches!(self, Self::Power | Self::Refinery)
    }

    fn candidates(self, controls: &AiControls) -> &[TypeId] {
        match self {
            Self::Power => &controls.build_power,
            Self::Refinery => &controls.build_refinery.candidates,
            Self::Barracks => &controls.build_barracks.candidates,
            Self::Weapons => &controls.build_weapons.candidates,
            Self::Radar => &controls.build_radar,
            Self::Tech => &controls.build_tech,
            Self::NavalYard => &controls.build_naval_yard,
            Self::Helipad => &controls.build_helipad.candidates,
            Self::Defense => &controls.build_defense.candidates,
            Self::Aa => &controls.build_aa.candidates,
        }
    }

    fn ratio_category(self, controls: &AiControls) -> Option<&AiBuildCategory> {
        match self {
            Self::Refinery => Some(&controls.build_refinery),
            Self::Barracks => Some(&controls.build_barracks),
            Self::Weapons => Some(&controls.build_weapons),
            Self::Helipad => Some(&controls.build_helipad),
            Self::Defense => Some(&controls.build_defense),
            Self::Aa => Some(&controls.build_aa),
            _ => None,
        }
    }
}

fn ai_category_order(include_army_factories: bool) -> Vec<AiCategory> {
    let mut steps = vec![AiCategory::Power, AiCategory::Refinery];
    if !include_army_factories {
        return steps;
    }
    steps.extend([
        AiCategory::Barracks,
        AiCategory::Weapons,
        AiCategory::Radar,
        AiCategory::Tech,
        AiCategory::NavalYard,
        AiCategory::Helipad,
        AiCategory::Defense,
        AiCategory::Aa,
    ]);
    steps
}

fn category_needed(world: &BattleState, house: &str, controls: &AiControls, kind: AiCategory) -> bool {
    let candidates = kind.candidates(controls);
    if candidates.is_empty() {
        return false;
    }
    match kind {
        AiCategory::Power => needs_power(world, house, controls),
        AiCategory::Radar | AiCategory::Tech | AiCategory::NavalYard => count_owned(world, house, candidates) == 0,
        _ => {
            let Some(cat) = kind.ratio_category(controls)
            else {
                return false;
            };
            needs_ratio_category(world, house, cat)
        }
    }
}

fn needs_power(world: &BattleState, house: &str, controls: &AiControls) -> bool {
    if count_owned(world, house, &controls.build_power) == 0 {
        return true;
    }
    let Some(player) = world.players.iter().find(|p| p.house.eq_ignore_ascii_case(house))
    else {
        return false;
    };
    if player.low_power() {
        return true;
    }
    let surplus = player.effective_power_output().saturating_sub(player.power_drain);
    surplus < controls.power_surplus
}

fn needs_ratio_category(world: &BattleState, house: &str, cat: &AiBuildCategory) -> bool {
    if cat.candidates.is_empty() {
        return false;
    }
    let owned = count_owned(world, house, &cat.candidates);
    if cat.limit > 0 && owned >= cat.limit {
        return false;
    }
    if owned == 0 {
        return true;
    }
    if cat.ratio_millis == 0 {
        return false;
    }
    let total = count_house_structures(world, house).max(1);
    let want = ((u64::from(total) * u64::from(cat.ratio_millis)) / 1000) as u32;
    let want = want.max(1);
    let want = if cat.limit > 0 { want.min(cat.limit) } else { want };
    owned < want
}

fn count_owned(world: &BattleState, house: &str, candidates: &[TypeId]) -> u32 {
    if candidates.is_empty() {
        return 0;
    }
    world
        .entities
        .iter()
        .filter(|e| {
            let id = e.id;
            !world.ecs_get::<Health>(id).map(|h| h.dead).unwrap_or(true)
                && world.ecs_get::<Owner>(id).map(|o| crate::gameplay::house_id_of(&world.definitions, house) == Some(o.house)).unwrap_or(false)
                && world
                    .ecs_get::<Identity>(id)
                    .map(|i| i.kind == MapEntityKind::Structure && candidates.iter().any(|c| *c == i.type_id))
                    .unwrap_or(false)
        })
        .count() as u32
}

fn count_house_structures(world: &BattleState, house: &str) -> u32 {
    world
        .entities
        .iter()
        .filter(|e| {
            let id = e.id;
            !world.ecs_get::<Health>(id).map(|h| h.dead).unwrap_or(true)
                && world.ecs_get::<Owner>(id).map(|o| crate::gameplay::house_id_of(&world.definitions, house) == Some(o.house)).unwrap_or(false)
                && world.ecs_get::<Identity>(id).map(|i| i.kind == MapEntityKind::Structure).unwrap_or(false)
        })
        .count() as u32
}

fn house_iq(world: &BattleState, house: &str) -> i32 {
    let from_map = world.prepared.houses.iter().find_map(|h| {
        let key = world.definitions.houses.get_by_id(h.country)?.type_key.as_str();
        if key.eq_ignore_ascii_case(house) || h.name.eq_ignore_ascii_case(house) { Some(h.iq) } else { None }
    });
    from_map.unwrap_or(world.definitions.ai_controls.max_iq_levels)
}

/// 房主 IQ 是否达到 rules `[IQ]` 阈值。
pub fn iq_allows(world: &BattleState, house: &str, threshold: i32) -> bool {
    house_iq(world, house) >= threshold
}

fn pick_from_candidates<'a>(world: &'a BattleState, house: &str, candidates: &[TypeId]) -> Option<&'a ra_types::StructureDefinition> {
    let living = living_structure_keys(world, house);
    let player = world.players.iter().find(|p| p.house.eq_ignore_ascii_case(house))?;
    let tech = TechTreePlayer::from_player(player);
    for &id in candidates {
        if !is_type_eligible_id(&world.definitions, tech, &living, id) {
            continue;
        }
        if let Some(s) = world.definitions.structures.get_by_id(id) {
            if !s.construction_yard {
                return Some(s);
            }
        }
    }
    None
}

/// 有空闲兵营时生产一名步兵。
pub fn produce_infantry_commands(world: &BattleState, house: &str, player: PlayerId) -> Vec<GameCommand> {
    if !iq_allows(world, house, world.definitions.ai_controls.iq_production) {
        return Vec::new();
    }
    if !house_has_idle_factory(world, house, ProductionCategory::Infantry) {
        return Vec::new();
    }
    let Some(unit) = pick_techno(world, house, ProductionCategory::Infantry)
    else {
        return Vec::new();
    };
    produce_unit(world, house, player, unit)
}

/// 有空闲战车工厂时生产一辆载具（非采矿车）。
pub fn produce_vehicle_commands(world: &BattleState, house: &str, player: PlayerId) -> Vec<GameCommand> {
    if !iq_allows(world, house, world.definitions.ai_controls.iq_production) {
        return Vec::new();
    }
    if !house_has_idle_factory(world, house, ProductionCategory::Vehicle) {
        return Vec::new();
    }
    let Some(unit) = pick_techno(world, house, ProductionCategory::Vehicle)
    else {
        return Vec::new();
    };
    produce_unit(world, house, player, unit)
}

/// 有矿场且无存活采矿车时，按 `[IQ] Harvester` 补一辆采矿车。
pub fn produce_harvester_commands(world: &BattleState, house: &str, player: PlayerId) -> Vec<GameCommand> {
    if !iq_allows(world, house, world.definitions.ai_controls.iq_harvester) {
        return Vec::new();
    }
    if !living_house_structure(world, house, |w, i| crate::gameplay::is_refinery(&w.definitions, i.type_id)) {
        return Vec::new();
    }
    if house_has_living_harvester(world, house) {
        return Vec::new();
    }
    if !house_has_idle_factory(world, house, ProductionCategory::Vehicle) {
        return Vec::new();
    }
    let Some(unit) = pick_harvester(world, house)
    else {
        return Vec::new();
    };
    produce_unit(world, house, player, unit)
}

/// 有空闲机场时按 `[IQ] Aircraft` 生产一架飞行器。
pub fn produce_aircraft_commands(world: &BattleState, house: &str, player: PlayerId) -> Vec<GameCommand> {
    if !iq_allows(world, house, world.definitions.ai_controls.iq_aircraft) {
        return Vec::new();
    }
    if !house_has_idle_factory(world, house, ProductionCategory::Aircraft) {
        return Vec::new();
    }
    let Some(unit) = pick_techno(world, house, ProductionCategory::Aircraft)
    else {
        return Vec::new();
    };
    produce_unit(world, house, player, unit)
}

fn produce_unit(world: &BattleState, house: &str, player: PlayerId, unit: &ra_types::TechnoDefinition) -> Vec<GameCommand> {
    let Some(funds) = world.house_funds(house)
    else {
        return Vec::new();
    };
    // 原版可零首付开单；AI 仅在完全没钱时跳过，避免空转排队。
    if funds <= 0 {
        return Vec::new();
    }
    vec![GameCommand::Produce { player, type_id: unit.id }]
}

fn queue_structure(world: &BattleState, house: &str, player: PlayerId, structure: &ra_types::StructureDefinition) -> Vec<GameCommand> {
    if !house_has_idle_yard(world, house) {
        return Vec::new();
    }
    let Some(funds) = world.house_funds(house)
    else {
        return Vec::new();
    };
    if funds <= 0 {
        return Vec::new();
    }
    vec![GameCommand::Produce { player, type_id: structure.id }]
}

fn place_structure(world: &BattleState, house: &str, player: PlayerId, structure: &ra_types::StructureDefinition) -> Vec<GameCommand> {
    // 当前未完成 Base 节点且类型吻合时，优先落在节点坐标。
    if let Some(node) = next_incomplete_base_node(world, house) {
        if node.type_id == structure.id {
            let gap = world.definitions.ai_base_spacing;
            if world.can_place_building_for_ai(house, structure.id, node.x, node.y, gap)
                || world.can_place_building_for_ai(house, structure.id, node.x, node.y, 0)
            {
                return vec![GameCommand::PlaceBuilding { player, type_id: structure.id, x: node.x, y: node.y }];
            }
        }
    }
    let Some((ox, oy)) = yard_placement_origin(world, house)
    else {
        return Vec::new();
    };
    let prefer_ore = structure.refinery;
    let Some((x, y)) = find_best_open(world, house, structure.id, ox, oy, prefer_ore)
    else {
        return Vec::new();
    };
    vec![GameCommand::PlaceBuilding { player, type_id: structure.id, x, y }]
}

fn base_plan_for_house<'a>(world: &'a BattleState, house: &str) -> Option<&'a ra_types::PreparedBasePlan> {
    let plan = world.prepared.base_plan.as_ref()?;
    let key = world.definitions.houses.get_by_id(plan.house)?.type_key.as_str();
    if key.eq_ignore_ascii_case(house) {
        Some(plan)
    }
    else {
        None
    }
}

fn next_incomplete_base_node(world: &BattleState, house: &str) -> Option<ra_types::PreparedBaseNode> {
    let plan = base_plan_for_house(world, house)?;
    for node in &plan.nodes {
        if !base_node_satisfied(world, house, node) {
            return Some(node.clone());
        }
    }
    None
}

fn base_node_satisfied(world: &BattleState, house: &str, node: &ra_types::PreparedBaseNode) -> bool {
    let (fw, fh) = world
        .definitions
        .structures
        .get_by_id(node.type_id)
        .map(|s| (s.foundation.width.max(1), s.foundation.height.max(1)))
        .unwrap_or((1, 1));
    world.entities.iter().any(|e| {
        let id = e.id;
        if world.ecs_get::<Health>(id).map(|h| h.dead).unwrap_or(true) {
            return false;
        }
        if !world.ecs_get::<Owner>(id).map(|o| crate::gameplay::house_id_of(&world.definitions, house) == Some(o.house)).unwrap_or(false) {
            return false;
        }
        let Some(identity) = world.ecs_get::<Identity>(id)
        else {
            return false;
        };
        if identity.kind != MapEntityKind::Structure || identity.type_id != node.type_id {
            return false;
        }
        let Some(xf) = world.ecs_get::<Transform>(id)
        else {
            return false;
        };
        // 节点落在该建筑 Foundation 内，或锚点恰好等于节点。
        let nx = i32::from(node.x);
        let ny = i32::from(node.y);
        let ax = i32::from(xf.x);
        let ay = i32::from(xf.y);
        nx >= ax && ny >= ay && nx < ax + i32::from(fw) && ny < ay + i32::from(fh)
    })
}

/// 有 `[Base]` 且节点已完成时：己方建筑数须小于「节点数 + BaseSizeAdd」才允许自由 AiControls 扩。
fn base_size_add_allows_freeform(world: &BattleState, house: &str) -> bool {
    let Some(plan) = base_plan_for_house(world, house)
    else {
        return true;
    };
    let cap = (plan.nodes.len() as u32).saturating_add(world.definitions.ai_controls.base_size_add);
    count_house_structures(world, house) < cap
}

/// 为指定阵营的空闲可攻击单位生成对最近敌军的 `Attack` 命令。
///
/// 无建造场时不下发（遭遇战开局仅有 MCV / 未展开时不应满图追打）。
pub fn auto_attack_commands(world: &BattleState, house: &str) -> Vec<GameCommand> {
    if !house_has_yard(world, house) {
        return Vec::new();
    }
    let mut out = Vec::new();
    for (attacker_index, attacker) in world.entities.iter().enumerate() {
        let id = attacker.id;
        if world.ecs_get::<Health>(id).map(|h| h.dead).unwrap_or(true) {
            continue;
        }
        if !world.ecs_get::<Owner>(id).map(|o| crate::gameplay::house_id_of(&world.definitions, house) == Some(o.house)).unwrap_or(false) {
            continue;
        }
        if world.ecs_get::<CombatStats>(id).map(|s| s.attack_damage == 0).unwrap_or(true) {
            continue;
        }
        if world.ecs_get::<AttackState>(id).map(|a| a.target.is_some()).unwrap_or(false) {
            continue;
        }
        if !world
            .ecs_get::<Identity>(id)
            .map(|i| matches!(i.kind, MapEntityKind::Unit | MapEntityKind::Infantry | MapEntityKind::Aircraft))
            .unwrap_or(false)
        {
            continue;
        }
        // 地图放置 `mission=Guard`：驻守，不参与 AI 主动追打。
        if world.ecs_get::<Identity>(id).map(|i| i.mission == Some(ra_types::MissionKind::Guard)).unwrap_or(false) {
            continue;
        }
        let Some(target_index) = nearest_enemy(world, attacker_index, house)
        else {
            continue;
        };
        out.push(GameCommand::Attack { attacker: id, target: world.entities[target_index].id });
    }
    out
}

fn pick_techno<'a>(world: &'a BattleState, house: &str, category: ProductionCategory) -> Option<&'a ra_types::TechnoDefinition> {
    let living = living_structure_keys(world, house);
    let Some(player) = world.players.iter().find(|p| p.house.eq_ignore_ascii_case(house))
    else {
        return None;
    };
    let tech = TechTreePlayer::from_player(player);
    world
        .definitions
        .techno
        .iter()
        .filter(|t| {
            t.class.production_category() == Some(category)
                && is_type_eligible_id(&world.definitions, tech, &living, t.id)
                && t.class != ra_types::TechnoClass::Building
                // 陆地工厂不造海军单位（否则 DEST 等会从战车厂刷出）。
                && !t.naval
                // 警犬等 Category=Dog 不进常规量产。
                && t.category != TechnoCategory::Dog
                && !t.harvester
                && deploy_into_type(&world.definitions, t.id).is_none()
        })
        // 同科技等级下优先较便宜的基础单位；再按类型键稳定排序。
        .min_by_key(|t| (t.tech_level, t.cost, t.type_key.as_str()))
}

fn pick_harvester<'a>(world: &'a BattleState, house: &str) -> Option<&'a ra_types::TechnoDefinition> {
    let living = living_structure_keys(world, house);
    let Some(player) = world.players.iter().find(|p| p.house.eq_ignore_ascii_case(house))
    else {
        return None;
    };
    let tech = TechTreePlayer::from_player(player);
    world
        .definitions
        .techno
        .iter()
        .filter(|t| {
            t.harvester
                && t.class.production_category() == Some(ProductionCategory::Vehicle)
                && is_type_eligible_id(&world.definitions, tech, &living, t.id)
                && !t.naval
        })
        .min_by_key(|t| (t.tech_level, t.cost, t.type_key.as_str()))
}

fn house_has_living_harvester(world: &BattleState, house: &str) -> bool {
    world.entities.iter().any(|e| {
        let id = e.id;
        if world.ecs_get::<Health>(id).map(|h| h.dead).unwrap_or(true) {
            return false;
        }
        if !world.ecs_get::<Owner>(id).map(|o| crate::gameplay::house_id_of(&world.definitions, house) == Some(o.house)).unwrap_or(false) {
            return false;
        }
        world.ecs_get::<Identity>(id).map(|i| crate::gameplay::is_harvester(&world.definitions, i.type_id)).unwrap_or(false)
    })
}

fn living_house_structure<'a, F>(world: &'a BattleState, house: &str, pred: F) -> bool
where
    F: Fn(&BattleState, &Identity) -> bool,
{
    world.entities.iter().any(|e| {
        let id = e.id;
        !world.ecs_get::<Health>(id).map(|h| h.dead).unwrap_or(true)
            && world.ecs_get::<Owner>(id).map(|o| crate::gameplay::house_id_of(&world.definitions, house) == Some(o.house)).unwrap_or(false)
            && world.ecs_get::<Identity>(id).map(|i| i.kind == MapEntityKind::Structure && pred(world, i)).unwrap_or(false)
    })
}

fn house_has_yard(world: &BattleState, house: &str) -> bool {
    living_house_structure(world, house, |w, i| is_construction_yard(&w.definitions, i.type_id))
}

fn house_has_idle_yard(world: &BattleState, house: &str) -> bool {
    world.entities.iter().any(|e| {
        let id = e.id;
        !world.ecs_get::<Health>(id).map(|h| h.dead).unwrap_or(true)
            && world.ecs_get::<Owner>(id).map(|o| crate::gameplay::house_id_of(&world.definitions, house) == Some(o.house)).unwrap_or(false)
            && world
                .ecs_get::<Identity>(id)
                .map(|i| i.kind == MapEntityKind::Structure && is_construction_yard(&world.definitions, i.type_id))
                .unwrap_or(false)
            && world.ecs_get::<ProductionQueue>(id).map(|q| q.item.is_none() && q.ready.is_none()).unwrap_or(false)
    })
}

fn house_has_idle_factory(world: &BattleState, house: &str, category: ProductionCategory) -> bool {
    world.entities.iter().any(|e| {
        let id = e.id;
        !world.ecs_get::<Health>(id).map(|h| h.dead).unwrap_or(true)
            && world.ecs_get::<Owner>(id).map(|o| crate::gameplay::house_id_of(&world.definitions, house) == Some(o.house)).unwrap_or(false)
            && world.ecs_get::<Identity>(id).map(|i| i.kind == MapEntityKind::Structure).unwrap_or(false)
            && world.ecs_get::<ProductionQueue>(id).map(|q| q.item.is_none()).unwrap_or(true)
            && world.ecs_get::<Identity>(id).map(|i| factory_matches_category(&world.definitions, i.type_id, category)).unwrap_or(false)
    })
}

fn yard_cell(world: &BattleState, house: &str) -> Option<(u16, u16, u16, u16)> {
    world.entities.iter().find_map(|e| {
        let id = e.id;
        if world.ecs_get::<Health>(id).map(|h| h.dead).unwrap_or(true) {
            return None;
        }
        if !world.ecs_get::<Owner>(id).map(|o| crate::gameplay::house_id_of(&world.definitions, house) == Some(o.house)).unwrap_or(false) {
            return None;
        }
        let identity = world.ecs_get::<Identity>(id)?;
        if identity.kind != MapEntityKind::Structure || !is_construction_yard(&world.definitions, identity.type_id) {
            return None;
        }
        let xf = world.ecs_get::<Transform>(id)?;
        let (fw, fh) = world
            .definitions
            .structures
            .get_by_id(identity.type_id)
            .map(|s| (s.foundation.width.max(1), s.foundation.height.max(1)))
            .unwrap_or((1, 1));
        Some((xf.x, xf.y, fw, fh))
    })
}

/// 建造场占地中心（用于落点搜索原点，避免只围着西北角扩）。
fn yard_placement_origin(world: &BattleState, house: &str) -> Option<(u16, u16)> {
    let (x, y, fw, fh) = yard_cell(world, house)?;
    Some((x.saturating_add((fw - 1) / 2), y.saturating_add((fh - 1) / 2)))
}

/// 在建造场附近按完整占地选最优可放格（左上角）。
///
/// - 搜索原点为建造场占地中心。
/// - 评分：默认靠近原点；矿场额外优先靠近可采矿。
/// - 间距仍走 `AIBaseSpacing` / `WantsExtraSpace`（及船厂最大距）。
fn find_best_open(world: &BattleState, house: &str, type_id: ra_types::TypeId, ox: u16, oy: u16, prefer_ore: bool) -> Option<(u16, u16)> {
    let sdef = world.definitions.structures.get_by_id(type_id);
    let base_gap = world.definitions.ai_base_spacing;
    let prefer_gap = if sdef.is_some_and(|s| s.wants_extra_space) { base_gap.saturating_add(1) } else { base_gap };
    let water_bound = sdef.is_some_and(|s| s.water_bound);
    let max_radius = if water_bound { world.definitions.ai_naval_yard_adjacency.max(1) as i32 } else { 16 };
    for &gap in &[prefer_gap, base_gap] {
        if let Some(cell) = find_best_open_with_gap(world, house, type_id, ox, oy, max_radius, gap, prefer_ore) {
            return Some(cell);
        }
        if gap == base_gap {
            break;
        }
    }
    None
}

fn find_best_open_with_gap(
    world: &BattleState,
    house: &str,
    type_id: ra_types::TypeId,
    ox: u16,
    oy: u16,
    max_radius: i32,
    min_gap_cells: u32,
    prefer_ore: bool,
) -> Option<(u16, u16)> {
    let (fw, fh) =
        world.definitions.structures.get_by_id(type_id).map(|s| (s.foundation.width.max(1), s.foundation.height.max(1))).unwrap_or((1, 1));
    let mut best: Option<(u32, u32, u16, u16)> = None;
    for dx in -max_radius..=max_radius {
        for dy in -max_radius..=max_radius {
            let x = i32::from(ox) + dx;
            let y = i32::from(oy) + dy;
            if x < 0 || y < 0 {
                continue;
            }
            let (x, y) = (x as u16, y as u16);
            if !world.can_place_building_for_ai(house, type_id, x, y, min_gap_cells) {
                continue;
            }
            let cx = x.saturating_add((fw - 1) / 2);
            let cy = y.saturating_add((fh - 1) / 2);
            let yard_dist = chebyshev_u16(cx, cy, ox, oy);
            let ore_dist = if prefer_ore {
                world.nearest_harvestable_ore(cx, cy).map(|(ore_x, ore_y)| chebyshev_u16(cx, cy, ore_x, ore_y)).unwrap_or(10_000)
            }
            else {
                0
            };
            // 矿场：先贴矿，再贴基地；其它：贴基地。再以坐标打破平局（稳定）。
            let key = if prefer_ore { (ore_dist, yard_dist, x, y) } else { (yard_dist, 0, x, y) };
            if best.is_none_or(|b| key < b) {
                best = Some(key);
            }
        }
    }
    best.map(|(_, _, x, y)| (x, y))
}

fn chebyshev_u16(ax: u16, ay: u16, bx: u16, by: u16) -> u32 {
    let dx = (i32::from(ax) - i32::from(bx)).unsigned_abs();
    let dy = (i32::from(ay) - i32::from(by)).unsigned_abs();
    dx.max(dy)
}

fn nearest_enemy(world: &BattleState, from: usize, house: &str) -> Option<usize> {
    let from_id = world.entities[from].id;
    let from_xf = world.ecs_get::<Transform>(from_id)?;
    let mut best: Option<(u32, usize)> = None;
    for (i, e) in world.entities.iter().enumerate() {
        if i == from {
            continue;
        }
        let id = e.id;
        if world.ecs_get::<Health>(id).map(|h| h.dead).unwrap_or(true) {
            continue;
        }
        if world.ecs_get::<Owner>(id).map(|o| crate::gameplay::house_id_of(&world.definitions, house) == Some(o.house)).unwrap_or(false) {
            continue;
        }
        if world
            .ecs_get::<Owner>(id)
            .map(|o| houses_are_allied(world, house, crate::gameplay::house_key_of(&world.definitions, o.house)))
            .unwrap_or(false)
        {
            continue;
        }
        if world.ecs_get::<Owner>(id).map(|o| is_ambient_house(&world.definitions, crate::gameplay::house_key_of(&world.definitions, o.house))).unwrap_or(false) {
            continue;
        }
        let Some(xf) = world.ecs_get::<Transform>(id)
        else {
            continue;
        };
        let dist = (i32::from(from_xf.x) - i32::from(xf.x)).unsigned_abs() + (i32::from(from_xf.y) - i32::from(xf.y)).unsigned_abs();
        if best.map(|(d, _)| dist < d).unwrap_or(true) {
            best = Some((dist, i));
        }
    }
    best.map(|(_, i)| i)
}
