//! 呈现脏集：供增量 `RenderWorld` 更新，避免每显示帧假定全表变更。

use ra_types::EntityId;

/// 自上次被呈现层消费以来变脏的实体 ID。
///
/// 权威 tick 内调用 [`DirtyEntitySet::mark`]；呈现侧在构建帧后 [`DirtyEntitySet::drain`]。
/// 当前仍可与全量 `snapshot` 并存：脏集是增量路径的契约，不是已完成的替换。
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct DirtyEntitySet {
    ids: Vec<EntityId>,
}

impl DirtyEntitySet {
    /// 空集。
    pub fn new() -> Self {
        Self::default()
    }

    /// 标记实体变脏（允许重复，drain 时去重）。
    pub fn mark(&mut self, id: EntityId) {
        self.ids.push(id);
    }

    /// 当前脏 ID（可能含重复）。
    pub fn as_slice(&self) -> &[EntityId] {
        &self.ids
    }

    /// 是否为空。
    pub fn is_empty(&self) -> bool {
        self.ids.is_empty()
    }

    /// 脏标记条数（含重复）。
    pub fn len(&self) -> usize {
        self.ids.len()
    }

    /// 取出并清空；结果已按 ID 排序去重。
    pub fn drain(&mut self) -> Vec<EntityId> {
        let mut out = std::mem::take(&mut self.ids);
        out.sort_unstable_by_key(|id| id.0);
        out.dedup();
        out
    }
}
