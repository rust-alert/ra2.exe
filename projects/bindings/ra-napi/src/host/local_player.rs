//! 本地玩家控制器：选中与点选指令（非权威状态）。

use ra_engine::Game;
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
    pub fn prune_dead(&mut self, game: &Game) {
        self.selected.retain(|&id| game.world.ecs_health(id).is_some_and(|(_, _, dead)| !dead));
    }

    /// 单选一个存活实体（单位或建筑）。
    pub fn select_only(&mut self, game: &Game, id: EntityId) {
        self.selected.clear();
        let Some((_, kind)) = game.world.ecs_identity(id)
        else {
            return;
        };
        let Some((_, _, dead)) = game.world.ecs_health(id)
        else {
            return;
        };
        if !dead && matches!(kind, MapEntityKind::Unit | MapEntityKind::Infantry | MapEntityKind::Aircraft | MapEntityKind::Structure) {
            self.selected.push(id);
        }
    }

    /// 若可多选则加入选中（已在选中则忽略）；与已选不同阵营则拒绝。
    pub fn select_add(&mut self, game: &Game, id: EntityId) {
        let Some((_, kind)) = game.world.ecs_identity(id)
        else {
            return;
        };
        let Some((_, _, dead)) = game.world.ecs_health(id)
        else {
            return;
        };
        if dead || !matches!(kind, MapEntityKind::Unit | MapEntityKind::Infantry | MapEntityKind::Aircraft | MapEntityKind::Structure) {
            return;
        }
        if let Some(&first) = self.selected.first() {
            let Some(first_owner) = game.world.ecs_owner(first)
            else {
                return;
            };
            let Some(owner) = game.world.ecs_owner(id)
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
    pub fn select_all_of_owner(&mut self, game: &Game, id: EntityId) {
        let Some(owner) = game.world.ecs_owner(id)
        else {
            return;
        };
        self.selected.clear();
        for eid in game.world.entity_ids() {
            let Some(o) = game.world.ecs_owner(eid)
            else {
                continue;
            };
            if o != owner {
                continue;
            }
            let Some((_, _, dead)) = game.world.ecs_health(eid)
            else {
                continue;
            };
            if dead {
                continue;
            }
            let Some((_, kind)) = game.world.ecs_identity(eid)
            else {
                continue;
            };
            if matches!(kind, MapEntityKind::Unit | MapEntityKind::Infantry | MapEntityKind::Aircraft) {
                self.selected.push(eid);
            }
        }
    }

    /// 在存活移动单位间循环选中。
    pub fn cycle_selection(&mut self, game: &Game) {
        let mobiles: Vec<EntityId> = game
            .world
            .entity_ids()
            .into_iter()
            .filter(|&eid| {
                game.world.ecs_health(eid).is_some_and(|(_, _, dead)| !dead)
                    && game
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
        self.select_only(game, next);
    }

    /// 清空选中。
    pub fn clear(&mut self) {
        self.selected.clear();
    }
}
