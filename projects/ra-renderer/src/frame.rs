//! 帧构建：引擎投影 → [`crate::world::RenderWorld`]（R1 骨架）。

use ra_engine::RenderSnapshot;

use crate::world::RenderWorld;

/// 将只读投影应用到可复用渲染世界。
///
/// 当前实现仍消费完整 [`RenderSnapshot`]（原型路径）。长期应改为脏实体 /
/// 事件流，并避免热路径上的字符串全表拷贝。
#[derive(Debug, Default)]
pub struct FrameBuilder;

impl FrameBuilder {
    /// 用完整快照刷新世界（过渡期 API；勿当作最终形态）。
    pub fn apply_full_snapshot(world: &mut RenderWorld, snap: &RenderSnapshot) {
        world.source_tick = snap.tick;
        world.dirty_count = snap.units.len() as u32;
    }
}
