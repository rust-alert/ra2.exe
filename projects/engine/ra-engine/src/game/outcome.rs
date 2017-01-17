use crate::{
    gameplay::{ai::is_ambient_house, is_base_unit},
    state::{
        BattleState,
        components::{Health, Identity, Owner},
    },
};
use ra_map::MapEntityKind;

use super::{session::BattleSession, types::SessionBootKind};

/// 对局结束结果。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BattleOutcome {
    /// 指定阵营获胜。
    Victory {
        /// 获胜阵营 owner 字符串。
        owner: String,
    },
    /// 本地或剧本判定失败（战役触发器 Lose、遭遇战本地出局等）。
    Defeat {
        /// 可选说明（触发器 id 等）。
        reason: String,
    },
}

/// 结算用统计（对局结束时锁定）。
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct BattleStats {
    /// 对局持续 tick。
    pub duration_ticks: u64,
    /// 已死亡移动单位数（全场）。
    pub units_lost: u32,
    /// 已死亡建筑数（全场）。
    pub buildings_lost: u32,
    /// 全场累计花费。
    pub funds_spent: i32,
    /// 各方结算行（顺序与开局玩家表一致）。
    pub players: Vec<PlayerBattleStats>,
}

/// 单方结算行（遭遇战积分表）。
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct PlayerBattleStats {
    /// 阵营 / house 名。
    pub house: String,
    /// 摧毁数（击杀记账）。
    pub kills: u32,
    /// 损失单位/建筑数。
    pub losses: u32,
    /// 建造/生产完成数。
    pub built: u32,
    /// 积分（临时：花费/100 + 摧毁×10 − 损失×5，下限 0）。
    pub score: i32,
}

impl BattleSession {
    /// 遭遇战：若仅剩一个阵营仍保活，进入 `SavourDelay` 收束后再锁定。
    /// 战役：消费触发器 `pending_outcome`，同样可走收束窗。
    pub(super) fn refresh_outcome(&mut self) {
        if self.outcome.is_some() {
            return;
        }
        if self.try_commit_savour() {
            return;
        }
        if let Some(outcome) = self.world.trigger_runtime.pending_outcome.take() {
            self.begin_savour(outcome);
            return;
        }
        if self.boot_kind == SessionBootKind::Campaign {
            return;
        }
        let Some(owner) = self.sole_victor().map(str::to_string)
        else {
            return;
        };
        let local_win = self
            .world
            .players
            .iter()
            .find(|p| p.id == self.world.local_player)
            .is_some_and(|p| p.house.as_ref().eq_ignore_ascii_case(&owner));
        let outcome = if local_win {
            BattleOutcome::Victory { owner: owner.clone() }
        } else {
            BattleOutcome::Defeat { reason: String::new() }
        };
        self.begin_savour(outcome);
    }

    /// 由剧本 / 触发器锁定胜负（战役主路径；仍经收束窗）。
    pub fn apply_scripted_outcome(&mut self, outcome: BattleOutcome) {
        if self.outcome.is_some() || self.pending_savour_outcome.is_some() {
            return;
        }
        self.begin_savour(outcome);
    }

    /// 开始 `SavourDelay`；延迟为 0 时立即锁定。
    fn begin_savour(&mut self, outcome: BattleOutcome) {
        if self.outcome.is_some() || self.pending_savour_outcome.is_some() {
            return;
        }
        let delay = u64::from(self.world.definitions.savour_delay_ticks);
        if delay == 0 {
            self.commit_outcome(outcome);
            return;
        }
        self.pending_savour_outcome = Some(outcome);
        self.savour_until_tick = Some(self.world.tick.saturating_add(delay));
        self.pause_reason = Some("胜负收束中".into());
    }

    /// 收束窗到期则写入 `outcome` 并暂停。已处理返回 `true`。
    fn try_commit_savour(&mut self) -> bool {
        let Some(until) = self.savour_until_tick
        else {
            return false;
        };
        if self.world.tick < until {
            return true;
        }
        let Some(outcome) = self.pending_savour_outcome.take()
        else {
            self.savour_until_tick = None;
            return true;
        };
        self.savour_until_tick = None;
        self.commit_outcome(outcome);
        true
    }

    fn commit_outcome(&mut self, outcome: BattleOutcome) {
        if self.outcome.is_some() {
            return;
        }
        self.battle_stats = Some(self.compute_battle_stats());
        let reason = match &outcome {
            BattleOutcome::Victory { owner } => {
                if self.boot_kind == SessionBootKind::Campaign {
                    format!("战役胜利 · {owner}")
                } else {
                    format!("胜负已定 · {owner}")
                }
            }
            BattleOutcome::Defeat { reason } => {
                if self.boot_kind == SessionBootKind::Campaign {
                    if reason.is_empty() {
                        "战役失败".into()
                    } else {
                        format!("战役失败 · {reason}")
                    }
                } else if reason.is_empty() {
                    "胜负已定".into()
                } else {
                    format!("胜负已定 · {reason}")
                }
            }
        };
        self.outcome = Some(outcome);
        self.paused = true;
        self.pause_reason = Some(reason);
    }

    fn compute_battle_stats(&self) -> BattleStats {
        let mut units_lost = 0u32;
        let mut buildings_lost = 0u32;
        let mut losses_by_house: std::collections::BTreeMap<String, u32> = std::collections::BTreeMap::new();
        for e in &self.world.entities {
            let id = e.id;
            if !self.world.ecs_get::<Health>(id).map(|h| h.dead).unwrap_or(false) {
                continue;
            }
            let house = self.world.ecs_get::<Owner>(id).map(|o| o.house.to_string()).unwrap_or_default();
            match self.world.ecs_get::<Identity>(id).map(|identity| identity.kind) {
                Some(MapEntityKind::Structure) => {
                    buildings_lost = buildings_lost.saturating_add(1);
                    if !house.is_empty() {
                        *losses_by_house.entry(house).or_default() += 1;
                    }
                }
                Some(MapEntityKind::Unit | MapEntityKind::Infantry | MapEntityKind::Aircraft) => {
                    units_lost = units_lost.saturating_add(1);
                    if !house.is_empty() {
                        *losses_by_house.entry(house).or_default() += 1;
                    }
                }
                _ => {}
            }
        }
        let funds_spent = self.world.players.iter().map(|p| p.funds_spent).sum();
        let players = self
            .world
            .players
            .iter()
            .map(|p| {
                let losses = losses_by_house.get(p.house.as_ref()).copied().unwrap_or(0);
                let kills = p.kills;
                let built = p.built;
                let score = (p.funds_spent / 100) + (kills as i32) * 10 - (losses as i32) * 5;
                PlayerBattleStats { house: p.house.to_string(), kills, losses, built, score: score.max(0) }
            })
            .collect();
        BattleStats { duration_ticks: self.world.tick, units_lost, buildings_lost, funds_spent, players }
    }

    /// 若仅剩一个非氛围阵营仍保活，返回其 owner。
    ///
    /// 至少需要两名非氛围玩家槽位，避免单机装载尚未开战时误判胜负。
    /// 短局：存活建筑或 `[General] BaseUnit` 保活。长局：任意存活建筑 / 步兵 / 载具 / 飞行器保活。
    pub fn sole_victor(&self) -> Option<&str> {
        let contenders: Vec<&str> = self
            .world
            .players
            .iter()
            .filter(|p| !is_ambient_house(p.house.as_ref()))
            .map(|p| p.house.as_ref())
            .collect();
        if contenders.len() < 2 {
            return None;
        }
        let alive: Vec<&str> = contenders
            .into_iter()
            .filter(|house| house_keeps_alive(&self.world, house, self.short_game))
            .collect();
        if alive.len() == 1 { Some(alive[0]) } else { None }
    }
}

/// 该 house 是否仍保活（未出局）。
fn house_keeps_alive(world: &BattleState, house: &str, short_game: bool) -> bool {
    for e in &world.entities {
        let id = e.id;
        if world.ecs_get::<Health>(id).map(|h| h.dead).unwrap_or(true) {
            continue;
        }
        if !world.ecs_get::<Owner>(id).map(|o| o.house.as_ref().eq_ignore_ascii_case(house)).unwrap_or(false) {
            continue;
        }
        let Some(identity) = world.ecs_get::<Identity>(id)
        else {
            continue;
        };
        match identity.kind {
            MapEntityKind::Structure => return true,
            MapEntityKind::Unit if short_game => {
                if is_base_unit(&world.definitions, &identity.type_id) {
                    return true;
                }
            }
            MapEntityKind::Unit | MapEntityKind::Infantry | MapEntityKind::Aircraft if !short_game => {
                return true;
            }
            _ => {}
        }
    }
    false
}
