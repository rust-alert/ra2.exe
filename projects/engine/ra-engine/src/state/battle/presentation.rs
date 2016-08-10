use super::types::BattleState;
use crate::presentation::DirtyEntitySet;
use ra_types::EntityId;

impl BattleState {
    /// 标记实体对呈现层变脏。
    pub fn mark_entity_dirty(&mut self, id: EntityId) {
        self.presentation_dirty.mark(id);
    }

    /// 只读查看当前脏集（可能含重复）。
    pub fn presentation_dirty(&self) -> &DirtyEntitySet {
        &self.presentation_dirty
    }

    /// 取出并清空脏集（排序去重），供帧构建消费。
    pub fn take_presentation_dirty(&mut self) -> Vec<EntityId> {
        self.presentation_dirty.drain()
    }
}
