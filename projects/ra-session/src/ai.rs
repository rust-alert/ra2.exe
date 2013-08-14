//! 基础 AI：只经 `GameCommand` 下发，不直接改写世界。

use ra_map::MapEntityKind;
use ra_types::PlayerId;
use ra_world::{GameCommand, World};

/// 为本阵营未部署的 MCV 生成 `Deploy`（已有建造场则跳过）。
pub fn deploy_mcv_commands(world: &World, house: &str) -> Vec<GameCommand> {
    if house_has_yard(world, house) {
        return Vec::new();
    }
    let mut out = Vec::new();
    for (entity_index, e) in world.entities.iter().enumerate() {
        if e.dead || e.owner != house {
            continue;
        }
        if !is_mcv(&e.type_id) {
            continue;
        }
        if e.kind != MapEntityKind::Unit {
            continue;
        }
        out.push(GameCommand::Deploy { entity_index });
    }
    out
}

/// 有建造场且无供电时，在建造场邻格放置一座电厂。
pub fn place_power_commands(world: &World, house: &str, player: PlayerId) -> Vec<GameCommand> {
    if !house_has_yard(world, house) || house_has_power(world, house) {
        return Vec::new();
    }
    let power_id = match side_for(world, house) {
        AiSide::Allied => "GAPOWR",
        AiSide::Soviet => "NAPOWR",
    };
    let Some(cost) = world.techno_cost(power_id)
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
    vec![GameCommand::PlaceBuilding {
        player,
        type_id: power_id.to_string(),
        x,
        y,
    }]
}

/// 为指定阵营的空闲可攻击单位生成对最近敌军的 `Attack` 命令。
pub fn auto_attack_commands(world: &World, house: &str) -> Vec<GameCommand> {
    let mut out = Vec::new();
    for (attacker_index, attacker) in world.entities.iter().enumerate() {
        if attacker.dead
            || attacker.owner != house
            || attacker.attack_damage == 0
            || attacker.attack_target.is_some()
            || !matches!(
                attacker.kind,
                MapEntityKind::Unit | MapEntityKind::Infantry | MapEntityKind::Aircraft
            )
        {
            continue;
        }
        let Some(target_index) = nearest_enemy(world, attacker_index, house)
        else {
            continue;
        };
        out.push(GameCommand::Attack { attacker_index, target_index });
    }
    out
}

#[derive(Clone, Copy)]
enum AiSide {
    Allied,
    Soviet,
}

fn side_for(world: &World, house: &str) -> AiSide {
    let soviet = world.entities.iter().any(|e| {
        e.owner == house
            && matches!(
                e.type_id.as_str(),
                "SMCV" | "NACNST" | "NAPOWR" | "NAHAND" | "NAWEAP" | "NAREFN" | "E2" | "HTNK"
            )
    });
    if soviet {
        AiSide::Soviet
    }
    else {
        AiSide::Allied
    }
}

fn house_has_yard(world: &World, house: &str) -> bool {
    world.entities.iter().any(|e| {
        !e.dead && e.owner == house && e.kind == MapEntityKind::Structure && is_yard(&e.type_id)
    })
}

fn house_has_power(world: &World, house: &str) -> bool {
    world.entities.iter().any(|e| {
        !e.dead && e.owner == house && e.kind == MapEntityKind::Structure && is_power(&e.type_id)
    })
}

fn yard_cell(world: &World, house: &str) -> Option<(u16, u16)> {
    world.entities.iter().find_map(|e| {
        if !e.dead && e.owner == house && e.kind == MapEntityKind::Structure && is_yard(&e.type_id) {
            Some((e.x, e.y))
        }
        else {
            None
        }
    })
}

fn find_open_near(world: &World, fx: u16, fy: u16) -> Option<(u16, u16)> {
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

fn is_mcv(type_id: &str) -> bool {
    matches!(type_id, "AMCV" | "SMCV")
}

fn is_yard(type_id: &str) -> bool {
    matches!(type_id, "GACNST" | "NACNST")
}

fn is_power(type_id: &str) -> bool {
    matches!(type_id, "GAPOWR" | "NAPOWR")
}

fn nearest_enemy(world: &World, from: usize, house: &str) -> Option<usize> {
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
