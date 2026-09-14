//! 建筑死亡 / 出售后的占地、电力、主厂与呈现拆除脏集。

use ra_types::{EntityId, MissionKind, TypeId};

use crate::state::{BattleState, components::Repairing};

impl BattleState {
    /// 建筑已逻辑死亡后的共用收尾：解占地、撤电力、主厂移交、排队拆除呈现。
    ///
    /// `mark_selling` 为真时写入 `MissionKind::Selling`（侧栏出售）；战损保持既有任务态。
    pub(crate) fn finalize_structure_removal(
        &mut self,
        building_id: EntityId,
        house: &str,
        type_id: TypeId,
        x: u16,
        y: u16,
        mark_selling: bool,
    ) {
        if mark_selling {
            let _ = self.with_identity_mut(building_id, |identity| {
                identity.mission = Some(MissionKind::Selling);
            });
        }
        if let Some(handle) = self.ecs.resolve(building_id) {
            let _ = self.ecs.world_mut().remove::<Repairing>(handle);
        }
        let foundation = self.definitions.structures.get_by_id(type_id).map(|s| s.foundation.clone()).unwrap_or_default();
        self.unseal_structure_footprint(x, y, foundation.width, foundation.height);
        self.revoke_structure_power(house, type_id);
        self.reassign_primary_after_factory_lost(building_id);
        if !self.structure_teardown_dirty.iter().any(|&id| id == building_id) {
            self.structure_teardown_dirty.push(building_id);
        }
    }
}
