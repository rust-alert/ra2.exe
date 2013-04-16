//! 确定性世界推进。不依赖渲染器与文件系统。

use ra_map::{MapEntityKind, MapInfo};
use ra_rules::{RulesDb, TechnoKind};
use ra_types::{GameEdition, PlayerId};

/// 走一格所需的移动点（预览用常量，非零售精确换算）。
pub const CELL_MOVE_COST: u32 = 64;

/// 世界中的一个已放置实体（由地图播种，后续仿真就地改）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorldEntity {
    pub kind: MapEntityKind,
    pub owner: String,
    pub type_id: String,
    pub x: u16,
    pub y: u16,
    pub facing: u8,
    pub sub_cell: u8,
    /// 当前生命（由放置段比例 × Strength）。
    pub health: u32,
    pub max_health: u32,
    pub speed: u32,
    pub techno_kind: Option<TechnoKind>,
    /// 简易移动目标格；无航点时为 `None`。
    pub target_x: Option<u16>,
    pub target_y: Option<u16>,
    /// 累积移动点（每 tick += Speed）。
    pub move_accum: u32,
}

#[derive(Debug, Clone)]
pub struct World {
    pub edition: GameEdition,
    pub tick: u64,
    pub map: MapInfo,
    pub entities: Vec<WorldEntity>,
    pub local_player: PlayerId,
    state_hash: u64,
}

impl World {
    pub fn new(edition: GameEdition, rules: &RulesDb, map: MapInfo) -> Self {
        let entities = map
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
                    sub_cell: e.sub_cell,
                    health,
                    max_health,
                    speed,
                    techno_kind: tt.map(|t| t.kind),
                    target_x,
                    target_y,
                    move_accum: 0,
                }
            })
            .collect();
        let mut world = Self {
            edition,
            tick: 0,
            map,
            entities,
            local_player: PlayerId(0),
            state_hash: 0,
        };
        world.rehash();
        world
    }

    pub fn advance_tick(&mut self) {
        self.tick = self.tick.wrapping_add(1);
        for e in &mut self.entities {
            if e.speed == 0 || !is_mobile(e.kind) {
                continue;
            }
            let (Some(tx), Some(ty)) = (e.target_x, e.target_y) else {
                continue;
            };
            if e.x == tx && e.y == ty {
                continue;
            }
            e.move_accum = e.move_accum.saturating_add(e.speed);
            while e.move_accum >= CELL_MOVE_COST {
                e.move_accum -= CELL_MOVE_COST;
                if !step_toward(e, tx, ty) {
                    break;
                }
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

/// 向目标迈一格；已到达则返回 `false`。
fn step_toward(e: &mut WorldEntity, tx: u16, ty: u16) -> bool {
    if e.x == tx && e.y == ty {
        return false;
    }
    let dx = i32::from(tx) - i32::from(e.x);
    let dy = i32::from(ty) - i32::from(e.y);
    if dx.abs() >= dy.abs() {
        if dx > 0 {
            e.x = e.x.saturating_add(1);
            e.facing = 0;
        } else {
            e.x = e.x.saturating_sub(1);
            e.facing = 128;
        }
    } else if dy > 0 {
        e.y = e.y.saturating_add(1);
        e.facing = 64;
    } else {
        e.y = e.y.saturating_sub(1);
        e.facing = 192;
    }
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

    #[test]
    fn binds_strength_and_speed() {
        let rules = rules_with_mtnk();
        let mut map = MapInfo::empty(GameEdition::Ra2, "t");
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
        let mut map = MapInfo::empty(GameEdition::Ra2, "t");
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
        // Speed=64 → 每 tick 正好走一格。
        world.advance_tick();
        assert_eq!(world.entities[0].x, 11);
        world.advance_tick();
        assert_eq!(world.entities[0].x, 12);
        world.advance_tick();
        assert_eq!(world.entities[0].x, 12);
    }
}
