use super::super::
components::CombatStats;
use super::types::BattleState;


impl BattleState {
    /// 当前确定性状态哈希（锁步校验用）。
    pub fn state_hash(&self) -> u64 {
        self.state_hash
    }

    /// 已绑定 techno 规则的实体数量。
    pub fn bound_techno_count(&self) -> usize {
        use crate::state::components::CombatStats;

        self.entities.iter().filter(|e| self.ecs_get::<CombatStats>(e.id).and_then(|s| s.techno_kind).is_some()).count()
    }
}
