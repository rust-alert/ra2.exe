//! 本地玩家控制器：选中与点选指令（非权威状态）。

use ra_engine::Game;
use ra_map::MapEntityKind;

/// 桌面本地玩家的 UI 选中与命令入口。
#[derive(Debug, Default, Clone)]
pub struct LocalPlayerController {
    /// 当前选中实体下标（对齐 `MatchState::entities`）。
    pub selected: Vec<usize>,
}

impl LocalPlayerController {
    /// 空控制器。
    pub fn new() -> Self {
        Self::default()
    }

    /// 去掉已死亡或不存在的选中项。
    pub fn prune_dead(&mut self, game: &Game) {
        self.selected.retain(|&i| i < game.world.entities.len() && !game.world.entities[i].dead);
    }

    /// 单选一个存活实体（单位或建筑）。
    pub fn select_only(&mut self, game: &Game, index: usize) {
        self.selected.clear();
        if index < game.world.entities.len()
            && !game.world.entities[index].dead
            && matches!(
                game.world.entities[index].kind,
                MapEntityKind::Unit | MapEntityKind::Infantry | MapEntityKind::Aircraft | MapEntityKind::Structure
            )
        {
            self.selected.push(index);
        }
    }

    /// 若可多选则加入选中（已在选中则忽略）；与已选不同阵营则拒绝。
    pub fn select_add(&mut self, game: &Game, index: usize) {
        if index >= game.world.entities.len() {
            return;
        }
        let e = &game.world.entities[index];
        if e.dead
            || !matches!(
                e.kind,
                MapEntityKind::Unit | MapEntityKind::Infantry | MapEntityKind::Aircraft | MapEntityKind::Structure
            )
        {
            return;
        }
        if let Some(&first) = self.selected.first() {
            if game.world.entities[first].owner != e.owner {
                return;
            }
        }
        if !self.selected.contains(&index) {
            self.selected.push(index);
        }
    }

    /// 选中与 `index` 同阵营的全部存活移动单位。
    pub fn select_all_of_owner(&mut self, game: &Game, index: usize) {
        if index >= game.world.entities.len() {
            return;
        }
        let owner = game.world.entities[index].owner.clone();
        self.selected.clear();
        for (i, e) in game.world.entities.iter().enumerate() {
            if e.dead || e.owner != owner {
                continue;
            }
            if matches!(e.kind, MapEntityKind::Unit | MapEntityKind::Infantry | MapEntityKind::Aircraft) {
                self.selected.push(i);
            }
        }
    }

    /// 在存活移动单位间循环选中。
    pub fn cycle_selection(&mut self, game: &Game) {
        let mobiles: Vec<usize> = game
            .world
            .entities
            .iter()
            .enumerate()
            .filter(|(_, e)| {
                !e.dead && matches!(e.kind, MapEntityKind::Unit | MapEntityKind::Infantry | MapEntityKind::Aircraft)
            })
            .map(|(i, _)| i)
            .collect();
        if mobiles.is_empty() {
            self.selected.clear();
            return;
        }
        let next = match self.selected.first() {
            Some(&cur) => {
                mobiles.iter().position(|&i| i == cur).map(|p| mobiles[(p + 1) % mobiles.len()]).unwrap_or(mobiles[0])
            }
            None => mobiles[0],
        };
        self.select_only(game, next);
    }

    /// 清空选中。
    pub fn clear(&mut self) {
        self.selected.clear();
    }
}
