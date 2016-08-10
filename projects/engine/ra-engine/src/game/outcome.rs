use crate::state::{
    BattleState,
    components::{Health, Identity, Owner},
};
use ra_map::MapEntityKind;
use ra_types::EntityId;

use super::{session::BattleSession, types::SessionBootKind};

/// 对局结束结果。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BattleOutcome {
    /// 指定阵营获胜。
    Victory {
        /// 获胜阵营 owner 字符串。
        owner: String,
    },
    /// 本地或剧本判定失败（战役触发器 Lose 等）。
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
    /// 遭遇战：若仅剩一个阵营仍有作战力量，锁定胜负并暂停。
    /// 战役：消费触发器 `pending_outcome`，不走 sole victor。
    pub(super) fn refresh_outcome(&mut self) {
        if self.outcome.is_some() {
            return;
        }
        if let Some(outcome) = self.world.trigger_runtime.pending_outcome.take() {
            self.apply_scripted_outcome(outcome);
            return;
        }
        if self.boot_kind == SessionBootKind::Campaign {
            return;
        }
        let Some(owner) = self.sole_victor().map(str::to_string)
        else {
            return;
        };
        self.battle_stats = Some(self.compute_battle_stats());
        self.outcome = Some(BattleOutcome::Victory { owner: owner.clone() });
        self.paused = true;
        self.pause_reason = Some(format!("胜负已定 · {owner}"));
    }

    /// 由剧本 / 触发器锁定胜负（战役主路径）。
    pub fn apply_scripted_outcome(&mut self, outcome: BattleOutcome) {
        if self.outcome.is_some() {
            return;
        }
        self.battle_stats = Some(self.compute_battle_stats());
        let reason = match &outcome {
            BattleOutcome::Victory { owner } => format!("战役胜利 · {owner}"),
            BattleOutcome::Defeat { reason } => {
                if reason.is_empty() {
                    "战役失败".into()
                }
                else {
                    format!("战役失败 · {reason}")
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

    /// 若仅剩一个阵营仍有作战力量（存活建筑或可作战移动单位），返回其 owner。
    /// 至少需要两名非氛围玩家槽位，避免单机装载尚未开战时误判胜负。
    /// `Neutral` / `Civilian` 氛围单位不计入作战力量。
    pub fn sole_victor(&self) -> Option<&str> {
        let skirmish_houses = self.world.players.iter().filter(|p| !crate::gameplay::ai::is_ambient_house(p.house.as_ref())).count();
        if skirmish_houses < 2 {
            return None;
        }
        let mut owners: Vec<&str> = self
            .world
            .entities
            .iter()
            .filter_map(|e| {
                let id = e.id;
                if !is_combat_force(&self.world, id) {
                    return None;
                }
                self.world.ecs_get::<Owner>(id).map(|o| o.house.as_ref())
            })
            .collect();
        owners.sort_unstable();
        owners.dedup();
        if owners.len() == 1 { Some(owners[0]) } else { None }
    }
}

/// 冻结胜负：存活建筑或可作战移动单位均算作战力量（排除 `Neutral` / `Civilian`）。
pub(super) fn is_combat_force(world: &BattleState, id: EntityId) -> bool {
    if world.ecs_get::<Health>(id).map(|h| h.dead).unwrap_or(true) {
        return false;
    }
    if world.ecs_get::<Owner>(id).map(|o| crate::gameplay::ai::is_ambient_house(o.house.as_ref())).unwrap_or(false) {
        return false;
    }
    world
        .ecs_get::<Identity>(id)
        .map(|identity| {
            matches!(identity.kind, MapEntityKind::Unit | MapEntityKind::Infantry | MapEntityKind::Aircraft | MapEntityKind::Structure)
        })
        .unwrap_or(false)
}
