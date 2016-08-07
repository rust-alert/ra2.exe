//! 命令条可视槽到 SHP 下标映射。

use crate::skin::text::{SKIRMISH_COMMAND_BAR, command_bar_shp_index};

pub(super) fn command_bar_shp_index_for_visual(visual_slot: usize) -> Option<usize> {
    let name = *SKIRMISH_COMMAND_BAR.get(visual_slot)?;
    command_bar_shp_index(name)
}
