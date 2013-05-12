//! 壳层共享的会话：持有世界、转发命令、产出呈现快照。
//!
//! 不碰文件系统与 GPU；桌面 / Web 只负责 I/O 与绘制。

use ra_map::MapEntityKind;
use ra_types::GameEdition;
use ra_world::{GameCommand, World};

/// 一帧呈现用的不可变快照（渲染器应逐步只消费此类数据）。
#[derive(Debug, Clone)]
pub struct RenderSnapshot {
    pub edition: GameEdition,
    pub tick: u64,
    pub state_hash: u64,
    pub units: Vec<SnapshotUnit>,
}

/// 快照中的一个可绘实体。
#[derive(Debug, Clone)]
pub struct SnapshotUnit {
    pub index: usize,
    pub kind: MapEntityKind,
    pub type_id: String,
    pub owner: String,
    pub x: u16,
    pub y: u16,
    pub facing: u8,
    pub turret_facing: u8,
    pub hva_frame: u16,
    pub health: u32,
    pub max_health: u32,
    pub dead: bool,
}

/// 运行中会话。
#[derive(Debug)]
pub struct Session {
    pub world: World,
    pub boot_note: String,
    /// 当前选中的实体下标（本地玩家操作）。
    pub selected: Vec<usize>,
}

impl Session {
    pub fn new(world: World, boot_note: impl Into<String>) -> Self {
        Self {
            world,
            boot_note: boot_note.into(),
            selected: Vec::new(),
        }
    }

    pub fn push_command(&mut self, cmd: GameCommand) {
        self.world.push_command(cmd);
    }

    pub fn tick(&mut self) {
        self.world.advance_tick();
        self.selected
            .retain(|&i| i < self.world.entities.len() && !self.world.entities[i].dead);
    }

    /// 单选一个存活移动单位。
    pub fn select_only(&mut self, index: usize) {
        self.selected.clear();
        if index < self.world.entities.len()
            && !self.world.entities[index].dead
            && matches!(
                self.world.entities[index].kind,
                MapEntityKind::Unit | MapEntityKind::Infantry | MapEntityKind::Aircraft
            )
        {
            self.selected.push(index);
        }
    }

    /// 在存活移动单位间循环选中。
    pub fn cycle_selection(&mut self) {
        let mobiles: Vec<usize> = self
            .world
            .entities
            .iter()
            .enumerate()
            .filter(|(_, e)| {
                !e.dead
                    && matches!(
                        e.kind,
                        MapEntityKind::Unit | MapEntityKind::Infantry | MapEntityKind::Aircraft
                    )
            })
            .map(|(i, _)| i)
            .collect();
        if mobiles.is_empty() {
            self.selected.clear();
            return;
        }
        let next = match self.selected.first() {
            Some(&cur) => mobiles
                .iter()
                .position(|&i| i == cur)
                .map(|p| mobiles[(p + 1) % mobiles.len()])
                .unwrap_or(mobiles[0]),
            None => mobiles[0],
        };
        self.select_only(next);
    }

    /// 选中单位移动到目标格。
    pub fn order_selected_move(&mut self, x: u16, y: u16) {
        for &i in &self.selected.clone() {
            self.world.push_command(GameCommand::MoveTo {
                entity_index: i,
                x,
                y,
            });
        }
    }

    /// 选中单位攻击目标。
    pub fn order_selected_attack(&mut self, target_index: usize) {
        for &i in &self.selected.clone() {
            if i != target_index {
                self.world.push_command(GameCommand::Attack {
                    attacker_index: i,
                    target_index,
                });
            }
        }
    }

    /// 相对 `from` 最近的异阵营存活移动单位。
    pub fn nearest_hostile(&self, from_index: usize) -> Option<usize> {
        let from = self.world.entities.get(from_index)?;
        if from.dead {
            return None;
        }
        self.world
            .entities
            .iter()
            .enumerate()
            .filter(|(j, e)| {
                *j != from_index
                    && !e.dead
                    && e.owner != from.owner
                    && matches!(
                        e.kind,
                        MapEntityKind::Unit | MapEntityKind::Infantry | MapEntityKind::Aircraft
                    )
            })
            .min_by_key(|(_, e)| {
                let dx = i32::from(e.x) - i32::from(from.x);
                let dy = i32::from(e.y) - i32::from(from.y);
                dx * dx + dy * dy
            })
            .map(|(j, _)| j)
    }

    /// 若仅剩一个阵营仍有存活移动单位，返回其 owner。
    pub fn sole_victor(&self) -> Option<&str> {
        let mut owners: Vec<&str> = self
            .world
            .entities
            .iter()
            .filter(|e| {
                !e.dead
                    && matches!(
                        e.kind,
                        MapEntityKind::Unit | MapEntityKind::Infantry | MapEntityKind::Aircraft
                    )
            })
            .map(|e| e.owner.as_str())
            .collect();
        owners.sort_unstable();
        owners.dedup();
        if owners.len() == 1 {
            Some(owners[0])
        } else {
            None
        }
    }

    pub fn snapshot(&self) -> RenderSnapshot {
        let units = self
            .world
            .entities
            .iter()
            .enumerate()
            .filter(|(_, e)| {
                matches!(
                    e.kind,
                    MapEntityKind::Unit | MapEntityKind::Infantry | MapEntityKind::Aircraft
                )
            })
            .map(|(index, e)| SnapshotUnit {
                index,
                kind: e.kind,
                type_id: e.type_id.clone(),
                owner: e.owner.clone(),
                x: e.x,
                y: e.y,
                facing: e.facing,
                turret_facing: e.turret_facing,
                hva_frame: e.hva_frame,
                health: e.health,
                max_health: e.max_health,
                dead: e.dead,
            })
            .collect();
        RenderSnapshot {
            edition: self.world.edition,
            tick: self.world.tick,
            state_hash: self.world.state_hash(),
            units,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ra_assets::IniDocument;
    use ra_map::{MapEntity, MapEntityKind, MapInfo, Waypoint};
    use ra_rules::{
        ColorSchemes, OverlayTypeRegistry, RulesDb, TechnoTypeRegistry,
    };
    use ra_types::GameEdition;
    use ra_world::GameCommand;

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
    fn session_tick_and_snapshot() {
        let rules = rules_with_mtnk();
        let mut map = MapInfo::empty(GameEdition::Ra2, "t");
        map.width = 20;
        map.height = 30;
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
        let world = World::new(GameEdition::Ra2, &rules, map);
        let mut session = Session::new(world, "test");
        session.push_command(GameCommand::MoveTo {
            entity_index: 0,
            x: 12,
            y: 10,
        });
        session.tick();
        let snap = session.snapshot();
        assert_eq!(snap.tick, 1);
        assert_eq!(snap.units.len(), 1);
        assert_eq!(snap.units[0].x, 11);
    }

    #[test]
    fn selection_orders_attack_and_detects_victor() {
        let rules = rules_with_mtnk();
        let mut map = MapInfo::empty(GameEdition::Ra2, "t");
        map.width = 20;
        map.height = 30;
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
            owner: "Russians".into(),
            type_id: "MTNK".into(),
            health: 256,
            x: 12,
            y: 10,
            facing: 0,
            sub_cell: 0,
        });
        let mut session = Session::new(World::new(GameEdition::Ra2, &rules, map), "t");
        session.world.entities[0].target_x = None;
        session.world.entities[0].target_y = None;
        session.world.entities[1].target_x = None;
        session.world.entities[1].target_y = None;
        session.world.entities[1].speed = 0;
        session.cycle_selection();
        assert_eq!(session.selected, vec![0]);
        let foe = session.nearest_hostile(0).unwrap();
        assert_eq!(foe, 1);
        session.order_selected_attack(foe);
        for _ in 0..80 {
            session.tick();
            if session.sole_victor().is_some() {
                break;
            }
        }
        assert_eq!(session.sole_victor(), Some("Americans"));
        assert!(session.world.entities[1].dead);
    }
}
