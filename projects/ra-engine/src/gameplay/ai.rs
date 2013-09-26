//! 基础 AI：只经 `GameCommand` 下发，不直接改写世界。
//!
//! 候选建筑 / 单位由冻结定义 + Owner 过滤选出，不硬编码外部类型名。

use crate::gameplay::{
    deploy_into_type, factory_matches_category, is_construction_yard, is_power_plant, is_refinery, owner_allows,
};
use crate::{GameCommand, MatchState};
use ra_map::MapEntityKind;
use ra_types::{ProductionCategory, PlayerId};

/// 为本阵营未部署的可部署单位生成 `Deploy`（已有建造场则跳过）。
pub fn deploy_mcv_commands(world: &MatchState, house: &str) -> Vec<GameCommand> {
    if house_has_yard(world, house) {
        return Vec::new();
    }
    let mut out = Vec::new();
    for e in world.entities.iter() {
        if e.dead || e.owner != house {
            continue;
        }
        if deploy_into_type(&world.definitions, &e.type_id).is_none() {
            continue;
        }
        if e.kind != MapEntityKind::Unit {
            continue;
        }
        out.push(GameCommand::Deploy { entity: e.id });
    }
    out
}

/// 有建造场且无供电时，在建造场邻格放置一座电厂。
pub fn place_power_commands(world: &MatchState, house: &str, player: PlayerId) -> Vec<GameCommand> {
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
pub fn place_barracks_commands(world: &MatchState, house: &str, player: PlayerId) -> Vec<GameCommand> {
    if !house_has_power(world, house) || house_has_factory(world, house, ProductionCategory::Infantry) {
        return Vec::new();
    }
    let Some(barracks_id) = pick_structure(world, house, |s| {
        s.production.as_ref().is_some_and(|p| p.category == ProductionCategory::Infantry)
    })
    else {
        return Vec::new();
    };
    place_near_yard(world, house, player, barracks_id)
}

/// 有供电且无战车工厂时，在建造场邻格放置一座战车工厂。
pub fn place_war_factory_commands(world: &MatchState, house: &str, player: PlayerId) -> Vec<GameCommand> {
    if !house_has_power(world, house) || house_has_factory(world, house, ProductionCategory::Vehicle) {
        return Vec::new();
    }
    let Some(weap_id) = pick_structure(world, house, |s| {
        s.production.as_ref().is_some_and(|p| p.category == ProductionCategory::Vehicle)
    })
    else {
        return Vec::new();
    };
    place_near_yard(world, house, player, weap_id)
}

/// 有供电且无矿场时，在建造场邻格放置一座矿场。
pub fn place_refinery_commands(world: &MatchState, house: &str, player: PlayerId) -> Vec<GameCommand> {
    if !house_has_power(world, house) || house_has_refinery(world, house) {
        return Vec::new();
    }
    let Some(refn_id) = pick_structure(world, house, |s| s.refinery)
    else {
        return Vec::new();
    };
    place_near_yard(world, house, player, refn_id)
}

/// 空闲兵营存在且资金足够时，排队生产步兵。
pub fn produce_infantry_commands(world: &MatchState, house: &str, player: PlayerId) -> Vec<GameCommand> {
    if !house_has_idle_factory(world, house, ProductionCategory::Infantry) {
        return Vec::new();
    }
    let Some(unit_id) = pick_techno(world, house, ProductionCategory::Infantry)
    else {
        return Vec::new();
    };
    produce_unit(world, house, player, unit_id)
}

/// 空闲战车工厂存在且资金足够时，排队生产载具。
pub fn produce_vehicle_commands(world: &MatchState, house: &str, player: PlayerId) -> Vec<GameCommand> {
    if !house_has_idle_factory(world, house, ProductionCategory::Vehicle) {
        return Vec::new();
    }
    let Some(unit_id) = pick_techno(world, house, ProductionCategory::Vehicle)
    else {
        return Vec::new();
    };
    produce_unit(world, house, player, unit_id)
}

fn produce_unit(world: &MatchState, house: &str, player: PlayerId, unit_id: &str) -> Vec<GameCommand> {
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

fn place_near_yard(world: &MatchState, house: &str, player: PlayerId, type_id: &str) -> Vec<GameCommand> {
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
pub fn auto_attack_commands(world: &MatchState, house: &str) -> Vec<GameCommand> {
    let mut out = Vec::new();
    for (attacker_index, attacker) in world.entities.iter().enumerate() {
        if attacker.dead
            || attacker.owner != house
            || attacker.attack_damage == 0
            || attacker.attack_target.is_some()
            || !matches!(attacker.kind, MapEntityKind::Unit | MapEntityKind::Infantry | MapEntityKind::Aircraft)
        {
            continue;
        }
        let Some(target_index) = nearest_enemy(world, attacker_index, house)
        else {
            continue;
        };
        out.push(GameCommand::Attack { attacker: attacker.id, target: world.entities[target_index].id });
    }
    out
}

fn pick_structure<'a, F>(world: &'a MatchState, house: &str, pred: F) -> Option<&'a str>
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

fn pick_techno<'a>(world: &'a MatchState, house: &str, category: ProductionCategory) -> Option<&'a str> {
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

fn house_has_yard(world: &MatchState, house: &str) -> bool {
    world.entities.iter().any(|e| {
        !e.dead && e.owner == house && e.kind == MapEntityKind::Structure && is_construction_yard(&world.definitions, &e.type_id)
    })
}

fn house_has_power(world: &MatchState, house: &str) -> bool {
    world
        .entities
        .iter()
        .any(|e| !e.dead && e.owner == house && e.kind == MapEntityKind::Structure && is_power_plant(&world.definitions, &e.type_id))
}

fn house_has_factory(world: &MatchState, house: &str, category: ProductionCategory) -> bool {
    world.entities.iter().any(|e| {
        !e.dead
            && e.owner == house
            && e.kind == MapEntityKind::Structure
            && factory_matches_category(&world.definitions, &e.type_id, category)
    })
}

fn house_has_idle_factory(world: &MatchState, house: &str, category: ProductionCategory) -> bool {
    world.entities.iter().any(|e| {
        !e.dead
            && e.owner == house
            && e.kind == MapEntityKind::Structure
            && e.produce_queue.is_none()
            && factory_matches_category(&world.definitions, &e.type_id, category)
    })
}

fn house_has_refinery(world: &MatchState, house: &str) -> bool {
    world
        .entities
        .iter()
        .any(|e| !e.dead && e.owner == house && e.kind == MapEntityKind::Structure && is_refinery(&world.definitions, &e.type_id))
}

fn yard_cell(world: &MatchState, house: &str) -> Option<(u16, u16)> {
    world.entities.iter().find_map(|e| {
        if !e.dead && e.owner == house && e.kind == MapEntityKind::Structure && is_construction_yard(&world.definitions, &e.type_id) {
            Some((e.x, e.y))
        }
        else {
            None
        }
    })
}

fn find_open_near(world: &MatchState, fx: u16, fy: u16) -> Option<(u16, u16)> {
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

fn nearest_enemy(world: &MatchState, from: usize, house: &str) -> Option<usize> {
    let a = &world.entities[from];
    let mut best: Option<(u32, usize)> = None;
    for (i, e) in world.entities.iter().enumerate() {
        if i == from || e.dead || e.owner == house {
            continue;
        }
        let dist = manhattan(a.x, a.y, e.x, e.y);
        if best.map(|(d, _)| dist < d).unwrap_or(true) {
            best = Some((dist, i));
        }
    }
    best.map(|(_, i)| i)
}

fn manhattan(ax: u16, ay: u16, bx: u16, by: u16) -> u32 {
    (i32::from(ax) - i32::from(bx)).unsigned_abs() + (i32::from(ay) - i32::from(by)).unsigned_abs()
}
