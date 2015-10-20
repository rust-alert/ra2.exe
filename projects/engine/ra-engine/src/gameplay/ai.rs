//! 基础 AI：只经 `GameCommand` 下发，不直接改写世界。
//!
//! 候选建筑 / 单位由冻结定义 + Owner 过滤选出，不硬编码外部类型名。

use crate::{
    GameCommand, BattleState,
    gameplay::{deploy_into_type, factory_matches_category, is_construction_yard, is_power_plant, is_refinery, owner_allows},
    state::components::{AttackState, CombatStats, Health, Identity, Owner, ProductionQueue, Transform},
};
use ra_map::MapEntityKind;
use ra_types::{PlayerId, ProductionCategory};

/// 地图氛围房主（平民装饰），不参与遭遇战 AI，也不计入胜负作战力量。
pub fn is_ambient_house(house: &str) -> bool {
    house.eq_ignore_ascii_case("Neutral") || house.eq_ignore_ascii_case("Civilian")
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
        if !world.ecs_get::<Owner>(id).map(|o| o.house.as_ref() == house).unwrap_or(false) {
            continue;
        }
        let Some(identity) = world.ecs_get::<Identity>(id)
        else {
            continue;
        };
        if deploy_into_type(&world.definitions, &identity.type_id).is_none() {
            continue;
        }
        if identity.kind != MapEntityKind::Unit {
            continue;
        }
        out.push(GameCommand::Deploy { entity: id });
    }
    out
}

/// 有建造场且无供电时，在建造场邻格放置一座电厂。
pub fn place_power_commands(world: &BattleState, house: &str, player: PlayerId) -> Vec<GameCommand> {
    if !house_has_yard(world, house) || house_has_power(world, house) {
        return Vec::new();
    }
    let Some(power_id) = pick_structure(world, house, |s| s.power.output > 0)
    else {
        return Vec::new();
    };
    place_near_yard(world, house, player, power_id)
}

/// 有供电且无兵营时，在建造场邻格放置一座兵营。
pub fn place_barracks_commands(world: &BattleState, house: &str, player: PlayerId) -> Vec<GameCommand> {
    if !house_has_power(world, house) || house_has_factory(world, house, ProductionCategory::Infantry) {
        return Vec::new();
    }
    let Some(barracks_id) = pick_structure(world, house, |s| s.production.as_ref().is_some_and(|p| p.category == ProductionCategory::Infantry))
    else {
        return Vec::new();
    };
    place_near_yard(world, house, player, barracks_id)
}

/// 有供电且无战车工厂时，在建造场邻格放置一座战车工厂。
pub fn place_war_factory_commands(world: &BattleState, house: &str, player: PlayerId) -> Vec<GameCommand> {
    if !house_has_power(world, house) || house_has_factory(world, house, ProductionCategory::Vehicle) {
        return Vec::new();
    }
    let Some(wf_id) = pick_structure(world, house, |s| s.production.as_ref().is_some_and(|p| p.category == ProductionCategory::Vehicle))
    else {
        return Vec::new();
    };
    place_near_yard(world, house, player, wf_id)
}

/// 有供电且无矿场时，在建造场邻格放置一座矿场。
pub fn place_refinery_commands(world: &BattleState, house: &str, player: PlayerId) -> Vec<GameCommand> {
    if !house_has_power(world, house) || house_has_refinery(world, house) {
        return Vec::new();
    }
    let Some(refinery_id) = pick_structure(world, house, |s| s.refinery)
    else {
        return Vec::new();
    };
    place_near_yard(world, house, player, refinery_id)
}

/// 有空闲兵营时生产一名步兵。
pub fn produce_infantry_commands(world: &BattleState, house: &str, player: PlayerId) -> Vec<GameCommand> {
    if !house_has_idle_factory(world, house, ProductionCategory::Infantry) {
        return Vec::new();
    }
    let Some(unit_id) = pick_techno(world, house, ProductionCategory::Infantry)
    else {
        return Vec::new();
    };
    produce_unit(world, house, player, unit_id)
}

/// 有空闲战车工厂时生产一辆载具。
pub fn produce_vehicle_commands(world: &BattleState, house: &str, player: PlayerId) -> Vec<GameCommand> {
    if !house_has_idle_factory(world, house, ProductionCategory::Vehicle) {
        return Vec::new();
    }
    let Some(unit_id) = pick_techno(world, house, ProductionCategory::Vehicle)
    else {
        return Vec::new();
    };
    produce_unit(world, house, player, unit_id)
}

fn produce_unit(world: &BattleState, house: &str, player: PlayerId, unit_id: &str) -> Vec<GameCommand> {
    let Some(cost) = world.techno_cost(unit_id)
    else {
        return Vec::new();
    };
    let Some(funds) = world.house_funds(house)
    else {
        return Vec::new();
    };
    if funds < cost as i32 {
        return Vec::new();
    }
    vec![GameCommand::Produce { player, type_id: unit_id.to_string() }]
}

fn place_near_yard(world: &BattleState, house: &str, player: PlayerId, type_id: &str) -> Vec<GameCommand> {
    let Some(cost) = world.techno_cost(type_id)
    else {
        return Vec::new();
    };
    let Some(funds) = world.house_funds(house)
    else {
        return Vec::new();
    };
    if funds < cost as i32 {
        return Vec::new();
    }
    let Some((yx, yy)) = yard_cell(world, house)
    else {
        return Vec::new();
    };
    let Some((x, y)) = find_open_near(world, yx, yy)
    else {
        return Vec::new();
    };
    vec![GameCommand::PlaceBuilding { player, type_id: type_id.to_string(), x, y }]
}

/// 为指定阵营的空闲可攻击单位生成对最近敌军的 `Attack` 命令。
pub fn auto_attack_commands(world: &BattleState, house: &str) -> Vec<GameCommand> {
    let mut out = Vec::new();
    for (attacker_index, attacker) in world.entities.iter().enumerate() {
        let id = attacker.id;
        if world.ecs_get::<Health>(id).map(|h| h.dead).unwrap_or(true) {
            continue;
        }
        if !world.ecs_get::<Owner>(id).map(|o| o.house.as_ref() == house).unwrap_or(false) {
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
        let Some(target_index) = nearest_enemy(world, attacker_index, house)
        else {
            continue;
        };
        out.push(GameCommand::Attack { attacker: id, target: world.entities[target_index].id });
    }
    out
}

fn pick_structure<'a, F>(world: &'a BattleState, house: &str, pred: F) -> Option<&'a str>
where
    F: Fn(&ra_types::StructureDefinition) -> bool,
{
    world
        .definitions
        .structures
        .iter()
        .filter(|s| pred(s) && owner_allows(&s.owner, house) && !s.construction_yard)
        .map(|s| s.type_key.as_str())
        .next()
}

fn pick_techno<'a>(world: &'a BattleState, house: &str, category: ProductionCategory) -> Option<&'a str> {
    world
        .definitions
        .techno
        .iter()
        .filter(|t| {
            t.class.production_category() == Some(category) && owner_allows(&t.owner, house) && t.class != ra_types::TechnoClass::Building
        })
        .map(|t| t.type_key.as_str())
        .next()
}

fn living_house_structure<'a, F>(world: &'a BattleState, house: &str, pred: F) -> bool
where
    F: Fn(&BattleState, &Identity) -> bool,
{
    world.entities.iter().any(|e| {
        let id = e.id;
        !world.ecs_get::<Health>(id).map(|h| h.dead).unwrap_or(true)
            && world.ecs_get::<Owner>(id).map(|o| o.house.as_ref() == house).unwrap_or(false)
            && world.ecs_get::<Identity>(id).map(|i| i.kind == MapEntityKind::Structure && pred(world, i)).unwrap_or(false)
    })
}

fn house_has_yard(world: &BattleState, house: &str) -> bool {
    living_house_structure(world, house, |w, i| is_construction_yard(&w.definitions, &i.type_id))
}

fn house_has_power(world: &BattleState, house: &str) -> bool {
    living_house_structure(world, house, |w, i| is_power_plant(&w.definitions, &i.type_id))
}

fn house_has_factory(world: &BattleState, house: &str, category: ProductionCategory) -> bool {
    living_house_structure(world, house, |w, i| factory_matches_category(&w.definitions, &i.type_id, category))
}

fn house_has_idle_factory(world: &BattleState, house: &str, category: ProductionCategory) -> bool {
    world.entities.iter().any(|e| {
        let id = e.id;
        !world.ecs_get::<Health>(id).map(|h| h.dead).unwrap_or(true)
            && world.ecs_get::<Owner>(id).map(|o| o.house.as_ref() == house).unwrap_or(false)
            && world.ecs_get::<Identity>(id).map(|i| i.kind == MapEntityKind::Structure).unwrap_or(false)
            && world.ecs_get::<ProductionQueue>(id).map(|q| q.item.is_none()).unwrap_or(true)
            && world
                .ecs_get::<Identity>(id)
                .map(|i| factory_matches_category(&world.definitions, &i.type_id, category))
                .unwrap_or(false)
    })
}

fn house_has_refinery(world: &BattleState, house: &str) -> bool {
    living_house_structure(world, house, |w, i| is_refinery(&w.definitions, &i.type_id))
}

fn yard_cell(world: &BattleState, house: &str) -> Option<(u16, u16)> {
    world.entities.iter().find_map(|e| {
        let id = e.id;
        if world.ecs_get::<Health>(id).map(|h| h.dead).unwrap_or(true) {
            return None;
        }
        if !world.ecs_get::<Owner>(id).map(|o| o.house.as_ref() == house).unwrap_or(false) {
            return None;
        }
        let identity = world.ecs_get::<Identity>(id)?;
        if identity.kind != MapEntityKind::Structure || !is_construction_yard(&world.definitions, &identity.type_id) {
            return None;
        }
        world.ecs_get::<Transform>(id).map(|t| (t.x, t.y))
    })
}

fn find_open_near(world: &BattleState, fx: u16, fy: u16) -> Option<(u16, u16)> {
    const DELTAS: [(i32, i32); 16] = [
        (1, 0),
        (0, 1),
        (-1, 0),
        (0, -1),
        (1, 1),
        (-1, 1),
        (-1, -1),
        (1, -1),
        (2, 0),
        (0, 2),
        (-2, 0),
        (0, -2),
        (2, 1),
        (1, 2),
        (-2, 1),
        (1, -2),
    ];
    for (dx, dy) in DELTAS {
        let x = i32::from(fx) + dx;
        let y = i32::from(fy) + dy;
        if x < 0 || y < 0 {
            continue;
        }
        let (x, y) = (x as u16, y as u16);
        if world.can_place_structure(x, y) {
            return Some((x, y));
        }
    }
    None
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
        if world.ecs_get::<Owner>(id).map(|o| o.house.as_ref() == house).unwrap_or(false) {
            continue;
        }
        if world.ecs_get::<Owner>(id).map(|o| is_ambient_house(o.house.as_ref())).unwrap_or(false) {
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
