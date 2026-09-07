//! 基础 AI：只经 `GameCommand` 下发，不直接改写世界。

use ra_map::MapEntityKind;
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

fn house_has_yard(world: &World, house: &str) -> bool {
    world.entities.iter().any(|e| {
        !e.dead && e.owner == house && e.kind == MapEntityKind::Structure && is_yard(&e.type_id)
    })
}

fn is_mcv(type_id: &str) -> bool {
    matches!(type_id, "AMCV" | "SMCV")
}

fn is_yard(type_id: &str) -> bool {
    matches!(type_id, "GACNST" | "NACNST")
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
