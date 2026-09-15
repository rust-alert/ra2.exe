//! 基础 AI：只经 `GameCommand` 下发，不直接改写世界。
//!
//! 候选建筑 / 单位由冻结定义 + Owner 过滤选出，不硬编码外部类型名。
//!
//! 基建单票顺序：供电（含低电补厂）→ 矿场 → 兵营 → 车厂。落点以建造场
//! 占地中心为原点；矿场额外优先靠近可采矿，并尊重 `AIBaseSpacing`。
//!
//! 有 `[AITriggerTypes]` 覆盖的房主：经济基建可由本模块补齐，作战部队交给
//! `tick_ai_triggers` → Create Team / ScriptTypes，避免与启发式工厂量产叠刷。

use crate::{
    BattleState, GameCommand,
    gameplay::{
        TechTreePlayer, deploy_into_type, factory_matches_category, is_construction_yard, is_power_plant, is_refinery, is_type_eligible_id,
        living_structure_keys,
    },
    state::components::{AttackState, CombatStats, Health, Identity, Owner, ProductionQueue, Transform},
};
use ra_map::MapEntityKind;
use ra_types::{PlayerId, ProductionCategory, TechnoCategory};

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
pub fn heuristic_should_produce_army(world: &BattleState, house: &str) -> bool {
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
pub fn is_ambient_house(house: &str) -> bool {
    house.eq_ignore_ascii_case("Neutral") || house.eq_ignore_ascii_case("Civilian") || house.eq_ignore_ascii_case("Special")
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
/// 基建表：供电 → 矿场 →（`include_army_factories`）兵营 → 车厂。
/// 若建造场已有完工件，则先落位该类型（不插队改造其它）。
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
    for step in ai_build_plan(include_army_factories) {
        if !step_needed(world, house, step) {
            continue;
        }
        let Some(structure) = pick_for_step(world, house, step)
        else {
            // 供电步若连候选都没有且仍无电厂，卡死后续；已有电厂仅低电时允许跳过。
            if matches!(step, AiBuildStep::Power) && !house_has_power(world, house) {
                return Vec::new();
            }
            continue;
        };
        return queue_structure(world, house, player, structure);
    }
    Vec::new()
}

/// 遭遇战启发式基建步骤（按表顺序单票推进）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum AiBuildStep {
    /// 无电或低电时补电厂。
    Power,
    /// 首座矿场。
    Refinery,
    /// 步兵工厂。
    InfantryFactory,
    /// 载具工厂。
    VehicleFactory,
}

fn ai_build_plan(include_army_factories: bool) -> Vec<AiBuildStep> {
    let mut steps = vec![AiBuildStep::Power, AiBuildStep::Refinery];
    if include_army_factories {
        steps.push(AiBuildStep::InfantryFactory);
        steps.push(AiBuildStep::VehicleFactory);
    }
    steps
}

fn step_needed(world: &BattleState, house: &str, step: AiBuildStep) -> bool {
    match step {
        AiBuildStep::Power => !house_has_power(world, house) || house_is_low_power(world, house),
        AiBuildStep::Refinery => !house_has_refinery(world, house),
        AiBuildStep::InfantryFactory => !house_has_factory(world, house, ProductionCategory::Infantry),
        AiBuildStep::VehicleFactory => !house_has_factory(world, house, ProductionCategory::Vehicle),
    }
}

fn pick_for_step<'a>(world: &'a BattleState, house: &str, step: AiBuildStep) -> Option<&'a ra_types::StructureDefinition> {
    match step {
        AiBuildStep::Power => pick_structure(world, house, |s| s.power.output > 0),
        AiBuildStep::Refinery => pick_structure(world, house, |s| s.refinery),
        AiBuildStep::InfantryFactory => {
            pick_structure(world, house, |s| s.production.as_ref().is_some_and(|p| p.category == ProductionCategory::Infantry))
        }
        AiBuildStep::VehicleFactory => {
            pick_structure(world, house, |s| s.production.as_ref().is_some_and(|p| p.category == ProductionCategory::Vehicle))
        }
    }
}

/// 有空闲兵营时生产一名步兵。
pub fn produce_infantry_commands(world: &BattleState, house: &str, player: PlayerId) -> Vec<GameCommand> {
    if !house_has_idle_factory(world, house, ProductionCategory::Infantry) {
        return Vec::new();
    }
    let Some(unit) = pick_techno(world, house, ProductionCategory::Infantry)
    else {
        return Vec::new();
    };
    produce_unit(world, house, player, unit)
}

/// 有空闲战车工厂时生产一辆载具。
pub fn produce_vehicle_commands(world: &BattleState, house: &str, player: PlayerId) -> Vec<GameCommand> {
    if !house_has_idle_factory(world, house, ProductionCategory::Vehicle) {
        return Vec::new();
    }
    let Some(unit) = pick_techno(world, house, ProductionCategory::Vehicle)
    else {
        return Vec::new();
    };
    produce_unit(world, house, player, unit)
}

fn produce_unit(world: &BattleState, house: &str, player: PlayerId, unit: &ra_types::TechnoDefinition) -> Vec<GameCommand> {
    let cost = unit.cost.max(0) as u32;
    let Some(funds) = world.house_funds(house)
    else {
        return Vec::new();
    };
    if funds < cost as i32 {
        return Vec::new();
    }
    vec![GameCommand::Produce { player, type_id: unit.id }]
}

fn queue_structure(world: &BattleState, house: &str, player: PlayerId, structure: &ra_types::StructureDefinition) -> Vec<GameCommand> {
    if !house_has_idle_yard(world, house) {
        return Vec::new();
    }
    let cost = if structure.cost > 0 {
        structure.cost.max(0) as u32
    }
    else {
        world.definitions.techno.get_by_id(structure.id).map(|t| t.cost.max(0) as u32).unwrap_or(0)
    };
    let Some(funds) = world.house_funds(house)
    else {
        return Vec::new();
    };
    if funds < cost as i32 {
        return Vec::new();
    }
    vec![GameCommand::Produce { player, type_id: structure.id }]
}

fn place_structure(world: &BattleState, house: &str, player: PlayerId, structure: &ra_types::StructureDefinition) -> Vec<GameCommand> {
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

fn pick_structure<'a, F>(world: &'a BattleState, house: &str, pred: F) -> Option<&'a ra_types::StructureDefinition>
where
    F: Fn(&ra_types::StructureDefinition) -> bool,
{
    let living = living_structure_keys(world, house);
    let Some(player) = world.players.iter().find(|p| p.house.eq_ignore_ascii_case(house))
    else {
        return None;
    };
    let tech = TechTreePlayer::from_player(player);
    world
        .definitions
        .structures
        .iter()
        .filter(|s| pred(s) && !s.construction_yard && is_type_eligible_id(&world.definitions, tech, &living, s.id))
        // 优先低科技、低造价的基础款（避免 `.find` 吃到反应堆/黑市等后置类型）。
        .min_by_key(|s| {
            let techno = world.definitions.techno.get_by_id(s.id);
            let tech_level = techno.map(|t| t.tech_level).unwrap_or(0);
            let cost = if s.cost > 0 { s.cost } else { techno.map(|t| t.cost).unwrap_or(0) };
            (tech_level, cost, s.type_key.as_str())
        })
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
                && deploy_into_type(&world.definitions, t.id).is_none()
        })
        // 同科技等级下优先较便宜的基础单位；再按类型键稳定排序。
        .min_by_key(|t| (t.tech_level, t.cost, t.type_key.as_str()))
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

fn house_has_power(world: &BattleState, house: &str) -> bool {
    living_house_structure(world, house, |w, i| is_power_plant(&w.definitions, i.type_id))
}

fn house_is_low_power(world: &BattleState, house: &str) -> bool {
    world.players.iter().find(|p| p.house.eq_ignore_ascii_case(house)).map(|p| p.low_power()).unwrap_or(false)
}

fn house_has_factory(world: &BattleState, house: &str, category: ProductionCategory) -> bool {
    living_house_structure(world, house, |w, i| factory_matches_category(&w.definitions, i.type_id, category))
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

fn house_has_refinery(world: &BattleState, house: &str) -> bool {
    living_house_structure(world, house, |w, i| is_refinery(&w.definitions, i.type_id))
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
        if world.ecs_get::<Owner>(id).map(|o| is_ambient_house(crate::gameplay::house_key_of(&world.definitions, o.house))).unwrap_or(false) {
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
