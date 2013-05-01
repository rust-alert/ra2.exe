//! 确定性世界推进。不依赖渲染器与文件系统。

use ra_map::{MapEntityKind, MapInfo, PassGrid};
use ra_rules::{RulesDb, TechnoKind};
use ra_types::{GameEdition, PlayerId};

/// 走一格所需的移动点（预览用常量，非零售精确换算）。
pub const CELL_MOVE_COST: u32 = 64;

/// 炮塔每 tick 最多转过的朝向单位（0..=255 环）。
pub const TURRET_TURN_STEP: u8 = 16;

/// 世界中的一个已放置实体（由地图播种，后续仿真就地改）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorldEntity {
    pub kind: MapEntityKind,
    pub owner: String,
    pub type_id: String,
    pub x: u16,
    pub y: u16,
    pub facing: u8,
    /// 炮塔朝向（无炮塔时与 `facing` 同步）。
    pub turret_facing: u8,
    pub sub_cell: u8,
    /// 当前生命（由放置段比例 × Strength）。
    pub health: u32,
    pub max_health: u32,
    pub speed: u32,
    pub techno_kind: Option<TechnoKind>,
    /// 简易移动目标格；无航点时为 `None`。
    pub target_x: Option<u16>,
    pub target_y: Option<u16>,
    /// 剩余路径（下一格在 `[0]`）。
    pub path: Vec<(u16, u16)>,
    /// 累积移动点（每 tick += Speed）。
    pub move_accum: u32,
}

#[derive(Debug, Clone)]
pub struct World {
    pub edition: GameEdition,
    pub tick: u64,
    pub map: MapInfo,
    pub pass_grid: PassGrid,
    pub entities: Vec<WorldEntity>,
    pub local_player: PlayerId,
    state_hash: u64,
}

impl World {
    pub fn new(edition: GameEdition, rules: &RulesDb, map: MapInfo) -> Self {
        let pass_grid = PassGrid::from_map(&map);
        let entities: Vec<WorldEntity> = map
            .entities
            .iter()
            .map(|e| {
                let tt = rules.techno_types.get(&e.type_id);
                let max_health = tt.map(|t| t.strength).unwrap_or(1).max(1);
                let health = (u64::from(max_health) * u64::from(e.health) / 256) as u32;
                let speed = tt.map(|t| t.speed).unwrap_or(0);
                let (target_x, target_y) = if is_mobile(e.kind) && speed > 0 {
                    nearest_waypoint(&map, e.x, e.y)
                } else {
                    (None, None)
                };
                WorldEntity {
                    kind: e.kind,
                    owner: e.owner.clone(),
                    type_id: e.type_id.clone(),
                    x: e.x,
                    y: e.y,
                    facing: e.facing,
                    turret_facing: e.facing,
                    sub_cell: e.sub_cell,
                    health,
                    max_health,
                    speed,
                    techno_kind: tt.map(|t| t.kind),
                    target_x,
                    target_y,
                    path: Vec::new(),
                    move_accum: 0,
                }
            })
            .collect();
        let mut world = Self {
            edition,
            tick: 0,
            map,
            pass_grid,
            entities,
            local_player: PlayerId(0),
            state_hash: 0,
        };
        for i in 0..world.entities.len() {
            repath_at(&mut world.entities, i, &world.pass_grid);
        }
        world.rehash();
        world
    }

    pub fn advance_tick(&mut self) {
        self.tick = self.tick.wrapping_add(1);
        let n = self.entities.len();
        for i in 0..n {
            if is_mobile(self.entities[i].kind) && self.entities[i].speed > 0 {
                if let (Some(tx), Some(ty)) =
                    (self.entities[i].target_x, self.entities[i].target_y)
                {
                    if self.entities[i].x == tx && self.entities[i].y == ty {
                        self.entities[i].path.clear();
                    } else {
                        self.entities[i].move_accum = self.entities[i]
                            .move_accum
                            .saturating_add(self.entities[i].speed);
                        while self.entities[i].move_accum >= CELL_MOVE_COST {
                            self.entities[i].move_accum -= CELL_MOVE_COST;
                            if self.entities[i].path.is_empty() {
                                repath_at(&mut self.entities, i, &self.pass_grid);
                                if self.entities[i].path.is_empty() {
                                    break;
                                }
                            }
                            let Some((nx, ny)) = self.entities[i].path.first().copied() else {
                                break;
                            };
                            if cell_occupied_by_other(&self.entities, i, nx, ny) {
                                // 下一格被占：清路再寻，仍堵则本 tick 停步。
                                self.entities[i].path.clear();
                                repath_at(&mut self.entities, i, &self.pass_grid);
                                let Some((nx2, ny2)) = self.entities[i].path.first().copied()
                                else {
                                    break;
                                };
                                if cell_occupied_by_other(&self.entities, i, nx2, ny2) {
                                    break;
                                }
                            }
                            if !step_along_path(&mut self.entities[i]) {
                                break;
                            }
                        }
                    }
                }
            }
            if is_mobile(self.entities[i].kind) {
                // 炮塔追车身朝向（攻击目标朝向另议）。
                let body = self.entities[i].facing;
                turn_facing_toward(
                    &mut self.entities[i].turret_facing,
                    body,
                    TURRET_TURN_STEP,
                );
            }
        }
        self.rehash();
    }

    pub fn state_hash(&self) -> u64 {
        self.state_hash
    }

    pub fn bound_techno_count(&self) -> usize {
        self.entities
            .iter()
            .filter(|e| e.techno_kind.is_some())
            .count()
    }

    /// 通行表变更后，为全部移动单位重算路径。
    pub fn repath_mobiles(&mut self) {
        for i in 0..self.entities.len() {
            if is_mobile(self.entities[i].kind) {
                repath_at(&mut self.entities, i, &self.pass_grid);
            }
        }
        self.rehash();
    }

    fn rehash(&mut self) {
        let mut h = self.tick;
        h = h
            .wrapping_mul(1099511628211)
            .wrapping_add(self.edition.as_str().len() as u64);
        h = h
            .wrapping_mul(1099511628211)
            .wrapping_add(self.entities.len() as u64);
        for e in &self.entities {
            h = h
                .wrapping_mul(1099511628211)
                .wrapping_add(e.x as u64)
                .wrapping_add((e.y as u64) << 16)
                .wrapping_add((e.facing as u64) << 32)
                .wrapping_add((e.turret_facing as u64) << 40)
                .wrapping_add(u64::from(e.health));
            for b in e.type_id.as_bytes() {
                h = h.wrapping_mul(1099511628211).wrapping_add(u64::from(*b));
            }
        }
        self.state_hash = h;
    }
}

fn is_mobile(kind: MapEntityKind) -> bool {
    matches!(
        kind,
        MapEntityKind::Unit | MapEntityKind::Infantry | MapEntityKind::Aircraft
    )
}

fn nearest_waypoint(map: &MapInfo, x: u16, y: u16) -> (Option<u16>, Option<u16>) {
    let Some(best) = map.waypoints.iter().min_by_key(|w| {
        let dx = i32::from(w.x) - i32::from(x);
        let dy = i32::from(w.y) - i32::from(y);
        dx * dx + dy * dy
    }) else {
        return (None, None);
    };
    (Some(best.x), Some(best.y))
}

fn turn_facing_toward(current: &mut u8, desired: u8, step: u8) {
    if *current == desired || step == 0 {
        return;
    }
    let cur = i16::from(*current);
    let want = i16::from(desired);
    let mut delta = (want - cur).rem_euclid(256);
    if delta > 128 {
        delta -= 256;
    }
    let step = i16::from(step);
    let moved = if delta > 0 {
        delta.min(step)
    } else {
        delta.max(-step)
    };
    *current = (cur + moved).rem_euclid(256) as u8;
}

fn cell_occupied_by_other(entities: &[WorldEntity], self_i: usize, x: u16, y: u16) -> bool {
    entities.iter().enumerate().any(|(j, o)| {
        j != self_i && is_mobile(o.kind) && o.x == x && o.y == y
    })
}

/// 寻路时把其它移动单位占格封死；目标被占则改停邻格。
fn repath_at(entities: &mut [WorldEntity], i: usize, grid: &PassGrid) {
    entities[i].path.clear();
    let (Some(tx), Some(ty)) = (entities[i].target_x, entities[i].target_y) else {
        return;
    };
    let (sx, sy) = (entities[i].x, entities[i].y);
    let mut g = grid.clone();
    for (j, o) in entities.iter().enumerate() {
        if j != i && is_mobile(o.kind) {
            g.set_passable(o.x, o.y, false);
        }
    }
    // 允许离开当前格。
    g.set_passable(sx, sy, true);
    let (gx, gy) = nearest_free_goal(&g, sx, sy, tx, ty);
    g.set_passable(gx, gy, true);
    let Some(mut path) = g.find_path_diag(sx, sy, gx, gy) else {
        return;
    };
    if path.first() == Some(&(sx, sy)) {
        path.remove(0);
    }
    entities[i].path = path;
}

/// 目标可走则用之；否则在半径内找最近可走格（含自身起点）。
fn nearest_free_goal(grid: &PassGrid, sx: u16, sy: u16, tx: u16, ty: u16) -> (u16, u16) {
    if grid.is_passable(tx, ty) {
        return (tx, ty);
    }
    let mut best: Option<(u32, u16, u16)> = None;
    for r in 1i32..=8 {
        for dy in -r..=r {
            for dx in -r..=r {
                if dx.abs() != r && dy.abs() != r {
                    continue;
                }
                let nx = i32::from(tx) + dx;
                let ny = i32::from(ty) + dy;
                if nx < 0 || ny < 0 {
                    continue;
                }
                let nx = nx as u16;
                let ny = ny as u16;
                if !grid.is_passable(nx, ny) {
                    continue;
                }
                // 允许停在自己脚下（已到邻格排队）。
                let dist = (i32::from(nx) - i32::from(sx)).pow(2)
                    + (i32::from(ny) - i32::from(sy)).pow(2);
                let key = (dist as u32, nx, ny);
                if best.map(|b| key < b).unwrap_or(true) {
                    best = Some(key);
                }
            }
        }
        if best.is_some() {
            break;
        }
    }
    best.map(|(_, x, y)| (x, y)).unwrap_or((tx, ty))
}

/// 沿 `path` 迈一格；无路则返回 `false`。
fn step_along_path(e: &mut WorldEntity) -> bool {
    let Some((nx, ny)) = e.path.first().copied() else {
        return false;
    };
    e.path.remove(0);
    let dx = i32::from(nx) - i32::from(e.x);
    let dy = i32::from(ny) - i32::from(e.y);
    e.facing = match (dx.signum(), dy.signum()) {
        (1, 0) => 0,
        (1, 1) => 32,
        (0, 1) => 64,
        (-1, 1) => 96,
        (-1, 0) => 128,
        (-1, -1) => 160,
        (0, -1) => 192,
        (1, -1) => 224,
        _ => e.facing,
    };
    e.x = nx;
    e.y = ny;
    true
}

#[cfg(test)]
mod tests {
    use super::*;
    use ra_assets::IniDocument;
    use ra_map::{MapEntity, Waypoint};
    use ra_rules::{
        ColorSchemes, OverlayTypeRegistry, RulesDb, TechnoKind, TechnoTypeRegistry,
    };
    use ra_types::GameEdition;

    fn rules_with_mtnk() -> RulesDb {
        let doc = IniDocument::parse(
            b"[VehicleTypes]\n0=MTNK\n\
[MTNK]\nStrength=400\nSpeed=64\nSight=6\nCost=800\nArmor=heavy\n",
        )
        .unwrap();
        RulesDb {
            edition: GameEdition::Ra2,
            rules: doc.clone(),
            art: IniDocument::default(),
            overlay_types: OverlayTypeRegistry::default(),
            color_schemes: ColorSchemes::default(),
            techno_types: TechnoTypeRegistry::from_rules(&doc),
        }
    }

    fn map_with_size() -> MapInfo {
        let mut map = MapInfo::empty(GameEdition::Ra2, "t");
        map.width = 20;
        map.height = 30;
        map
    }

    #[test]
    fn binds_strength_and_speed() {
        let rules = rules_with_mtnk();
        let mut map = map_with_size();
        map.entities.push(MapEntity {
            kind: MapEntityKind::Unit,
            owner: "Americans".into(),
            type_id: "MTNK".into(),
            health: 256,
            x: 10,
            y: 20,
            facing: 64,
            sub_cell: 0,
        });
        let world = World::new(GameEdition::Ra2, &rules, map);
        assert_eq!(world.entities.len(), 1);
        let e = &world.entities[0];
        assert_eq!(e.max_health, 400);
        assert_eq!(e.health, 400);
        assert_eq!(e.speed, 64);
        assert_eq!(e.techno_kind, Some(TechnoKind::Vehicle));
        assert_eq!(world.bound_techno_count(), 1);
    }

    #[test]
    fn advances_toward_waypoint() {
        let rules = rules_with_mtnk();
        let mut map = map_with_size();
        map.waypoints.push(Waypoint {
            index: 0,
            x: 12,
            y: 20,
        });
        map.entities.push(MapEntity {
            kind: MapEntityKind::Unit,
            owner: "Americans".into(),
            type_id: "MTNK".into(),
            health: 256,
            x: 10,
            y: 20,
            facing: 0,
            sub_cell: 0,
        });
        let mut world = World::new(GameEdition::Ra2, &rules, map);
        assert_eq!(world.entities[0].target_x, Some(12));
        assert_eq!(world.entities[0].path.len(), 2);
        world.advance_tick();
        assert_eq!(world.entities[0].x, 11);
        world.advance_tick();
        assert_eq!(world.entities[0].x, 12);
        world.advance_tick();
        assert_eq!(world.entities[0].x, 12);
    }

    #[test]
    fn bfs_detours_around_structure() {
        let rules = rules_with_mtnk();
        let mut map = map_with_size();
        map.waypoints.push(Waypoint {
            index: 0,
            x: 14,
            y: 10,
        });
        map.entities.push(MapEntity {
            kind: MapEntityKind::Structure,
            owner: "Neutral".into(),
            type_id: "GAWALL".into(),
            health: 256,
            x: 12,
            y: 10,
            facing: 0,
            sub_cell: 0,
        });
        map.entities.push(MapEntity {
            kind: MapEntityKind::Unit,
            owner: "Americans".into(),
            type_id: "MTNK".into(),
            health: 256,
            x: 10,
            y: 10,
            facing: 0,
            sub_cell: 0,
        });
        let mut world = World::new(GameEdition::Ra2, &rules, map);
        assert!(!world.pass_grid.is_passable(12, 10));
        assert!(!world.entities[1].path.is_empty());
        assert!(!world.entities[1].path.iter().any(|&(x, y)| x == 12 && y == 10));
        // 八邻绕行仍短于直线穿墙，且不踩封死格。
        assert!(!world.entities[1].path.is_empty());
        assert!(world.entities[1].path.len() >= 3);
        for _ in 0..20 {
            world.advance_tick();
        }
        assert_eq!(world.entities[1].x, 14);
        assert_eq!(world.entities[1].y, 10);
    }

    #[test]
    fn mobiles_detour_around_each_other() {
        let rules = rules_with_mtnk();
        let mut map = map_with_size();
        map.waypoints.push(Waypoint {
            index: 0,
            x: 14,
            y: 10,
        });
        // 挡在直线上的静止单位（Speed=0 用建筑外的占格：另一辆坦克无目标则不移动）。
        map.entities.push(MapEntity {
            kind: MapEntityKind::Unit,
            owner: "Americans".into(),
            type_id: "MTNK".into(),
            health: 256,
            x: 12,
            y: 10,
            facing: 0,
            sub_cell: 0,
        });
        map.entities.push(MapEntity {
            kind: MapEntityKind::Unit,
            owner: "Americans".into(),
            type_id: "MTNK".into(),
            health: 256,
            x: 10,
            y: 10,
            facing: 0,
            sub_cell: 0,
        });
        let mut world = World::new(GameEdition::Ra2, &rules, map);
        // 两车都朝同一航点；后者路径不得踩前者当前格。
        assert!(!world.entities[1].path.is_empty());
        assert!(!world.entities[1]
            .path
            .iter()
            .any(|&(x, y)| x == world.entities[0].x && y == world.entities[0].y));
        for _ in 0..40 {
            world.advance_tick();
        }
        // 至少一车抵达或贴近航点；且不同时占同一格。
        let a = (world.entities[0].x, world.entities[0].y);
        let b = (world.entities[1].x, world.entities[1].y);
        assert_ne!(a, b);
        assert!(a == (14, 10) || b == (14, 10) || a.0.max(b.0) >= 13);
    }

    #[test]
    fn shared_waypoint_queues_on_neighbor() {
        let rules = rules_with_mtnk();
        let mut map = map_with_size();
        map.waypoints.push(Waypoint {
            index: 0,
            x: 12,
            y: 10,
        });
        map.entities.push(MapEntity {
            kind: MapEntityKind::Unit,
            owner: "Americans".into(),
            type_id: "MTNK".into(),
            health: 256,
            x: 10,
            y: 10,
            facing: 0,
            sub_cell: 0,
        });
        map.entities.push(MapEntity {
            kind: MapEntityKind::Unit,
            owner: "Americans".into(),
            type_id: "MTNK".into(),
            health: 256,
            x: 10,
            y: 11,
            facing: 0,
            sub_cell: 0,
        });
        let mut world = World::new(GameEdition::Ra2, &rules, map);
        for _ in 0..30 {
            world.advance_tick();
        }
        let a = (world.entities[0].x, world.entities[0].y);
        let b = (world.entities[1].x, world.entities[1].y);
        assert_ne!(a, b);
        // 一车占航点，另一车停在曼哈顿距离 ≤2 的邻域。
        let on_wp = |p: (u16, u16)| p == (12, 10);
        assert!(on_wp(a) || on_wp(b));
        let other = if on_wp(a) { b } else { a };
        let dist = (i32::from(other.0) - 12).unsigned_abs() + (i32::from(other.1) - 10).unsigned_abs();
        assert!(dist <= 2);
    }

    #[test]
    fn turret_chases_body_facing() {
        let rules = rules_with_mtnk();
        let mut map = map_with_size();
        map.waypoints.push(Waypoint {
            index: 0,
            x: 12,
            y: 20,
        });
        map.entities.push(MapEntity {
            kind: MapEntityKind::Unit,
            owner: "Americans".into(),
            type_id: "MTNK".into(),
            health: 256,
            x: 10,
            y: 20,
            facing: 0,
            sub_cell: 0,
        });
        let mut world = World::new(GameEdition::Ra2, &rules, map);
        world.entities[0].turret_facing = 128;
        world.advance_tick();
        // 车身迈步后 facing 变；炮塔每 tick 最多转 TURRET_TURN_STEP。
        let body = world.entities[0].facing;
        let tur = world.entities[0].turret_facing;
        assert_ne!(tur, 128);
        let delta = (i16::from(body) - i16::from(tur)).rem_euclid(256);
        let shortest = if delta > 128 { 256 - delta } else { delta };
        assert!(shortest < 128);
    }
}
