//! 本地玩家控制器：选中与点选指令（非权威状态）。

use ra_engine::BattleSession;
use ra_map::MapEntityKind;
use ra_types::EntityId;

/// 桌面本地玩家的 UI 选中与命令入口。
#[derive(Debug, Default, Clone)]
pub struct LocalPlayerController {
    /// 当前选中实体的稳定 ID。
    pub selected: Vec<EntityId>,
}

impl LocalPlayerController {
    /// 空控制器。
    pub fn new() -> Self {
        Self::default()
    }

    /// 去掉已死亡或不存在的选中项。
    pub fn prune_dead(&mut self, battle: &BattleSession) {
        self.selected.retain(|&id| battle.world.ecs_health(id).is_some_and(|(_, _, dead)| !dead));
    }

    /// 单选一个存活实体（单位或建筑）。
    pub fn select_only(&mut self, battle: &BattleSession, id: EntityId) {
        self.selected.clear();
        let Some((_, kind)) = battle.world.ecs_identity(id)
        else {
            return;
        };
        let Some((_, _, dead)) = battle.world.ecs_health(id)
        else {
            return;
        };
        if !dead && matches!(kind, MapEntityKind::Unit | MapEntityKind::Infantry | MapEntityKind::Aircraft | MapEntityKind::Structure) {
            self.selected.push(id);
        }
    }

    /// 若可多选则加入选中（已在选中则忽略）；与已选不同阵营则拒绝。
    pub fn select_add(&mut self, battle: &BattleSession, id: EntityId) {
        let Some((_, kind)) = battle.world.ecs_identity(id)
        else {
            return;
        };
        let Some((_, _, dead)) = battle.world.ecs_health(id)
        else {
            return;
        };
        if dead || !matches!(kind, MapEntityKind::Unit | MapEntityKind::Infantry | MapEntityKind::Aircraft | MapEntityKind::Structure) {
            return;
        }
        if let Some(&first) = self.selected.first() {
            let Some(first_owner) = battle.world.ecs_owner(first)
            else {
                return;
            };
            let Some(owner) = battle.world.ecs_owner(id)
            else {
                return;
            };
            if first_owner != owner {
                return;
            }
        }
        if !self.selected.contains(&id) {
            self.selected.push(id);
        }
    }

    /// 选中与 `id` 同阵营的全部存活移动单位。
    pub fn select_all_of_owner(&mut self, battle: &BattleSession, id: EntityId) {
        let Some(owner) = battle.world.ecs_owner(id)
        else {
            return;
        };
        self.selected.clear();
        for eid in battle.world.entity_ids() {
            let Some(o) = battle.world.ecs_owner(eid)
            else {
                continue;
            };
            if o != owner {
                continue;
            }
            let Some((_, _, dead)) = battle.world.ecs_health(eid)
            else {
                continue;
            };
            if dead {
                continue;
            }
            let Some((_, kind)) = battle.world.ecs_identity(eid)
            else {
                continue;
            };
            if matches!(kind, MapEntityKind::Unit | MapEntityKind::Infantry | MapEntityKind::Aircraft) {
                self.selected.push(eid);
            }
        }
    }

    /// 选中与当前首个选中项同类型、同阵营的全部存活移动单位（原版 `T`）。
    ///
    /// 无选中时不改变选中集。
    pub fn select_same_type(&mut self, battle: &BattleSession) {
        let Some(&seed) = self.selected.first()
        else {
            return;
        };
        let Some(owner) = battle.world.ecs_owner(seed)
        else {
            return;
        };
        let Some((type_id, kind)) = battle.world.ecs_identity(seed)
        else {
            return;
        };
        if !matches!(kind, MapEntityKind::Unit | MapEntityKind::Infantry | MapEntityKind::Aircraft) {
            return;
        }
        self.selected.clear();
        for eid in battle.world.entity_ids() {
            if battle.world.ecs_health(eid).is_none_or(|(_, _, dead)| dead) {
                continue;
            }
            if battle.world.ecs_owner(eid).is_none_or(|o| o != owner) {
                continue;
            }
            let Some((tid, k)) = battle.world.ecs_identity(eid)
            else {
                continue;
            };
            if k != kind {
                continue;
            }
            if tid.as_ref() != type_id.as_ref() {
                continue;
            }
            self.selected.push(eid);
        }
    }

    /// 在本地玩家存活移动单位间循环选中（`Tab`）。
    pub fn cycle_selection(&mut self, battle: &BattleSession) {
        let Some(local_house) = battle.world.players.iter().find(|p| p.id == battle.world.local_player).map(|p| p.house.clone())
        else {
            self.selected.clear();
            return;
        };
        let mobiles: Vec<EntityId> = battle
            .world
            .entity_ids()
            .into_iter()
            .filter(|&eid| {
                battle.world.ecs_health(eid).is_some_and(|(_, _, dead)| !dead)
                    && battle.world.ecs_owner(eid).is_some_and(|o| o.as_ref() == local_house.as_ref())
                    && battle
                        .world
                        .ecs_identity(eid)
                        .is_some_and(|(_, kind)| matches!(kind, MapEntityKind::Unit | MapEntityKind::Infantry | MapEntityKind::Aircraft))
            })
            .collect();
        if mobiles.is_empty() {
            self.selected.clear();
            return;
        }
        let next = match self.selected.first() {
            Some(&cur) => mobiles.iter().position(|&i| i == cur).map(|p| mobiles[(p + 1) % mobiles.len()]).unwrap_or(mobiles[0]),
            None => mobiles[0],
        };
        self.select_only(battle, next);
    }

    /// 选中本地开局单位（优先 MCV）。
    pub fn select_local_start(&mut self, battle: &BattleSession) -> Option<EntityId> {
        let id = battle.local_start_mobile()?;
        self.select_only(battle, id);
        Some(id)
    }

    /// 用给定实体集合替换选中（仅保留存活的本方可控移动单位与建筑）。
    ///
    /// `add=true` 时在现有选中上追加（跨阵营仍拒绝）。
    pub fn apply_ids(&mut self, battle: &BattleSession, ids: &[EntityId], add: bool) {
        if !add {
            self.selected.clear();
        }
        for &id in ids {
            if add {
                self.select_add(battle, id);
            }
            else {
                let Some((_, kind)) = battle.world.ecs_identity(id)
                else {
                    continue;
                };
                let Some((_, _, dead)) = battle.world.ecs_health(id)
                else {
                    continue;
                };
                if dead
                    || !matches!(
                        kind,
                        MapEntityKind::Unit | MapEntityKind::Infantry | MapEntityKind::Aircraft | MapEntityKind::Structure
                    )
                {
                    continue;
                }
                if !self.selected.contains(&id) {
                    self.selected.push(id);
                }
            }
        }
    }

    /// 清空选中。
    pub fn clear(&mut self) {
        self.selected.clear();
    }
}
