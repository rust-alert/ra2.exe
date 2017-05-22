//! 对局通行层最终封口：Foundation 种子之后的 TMP → overlay land。

use ra_types::{AssetSource, OverlayTypeRegistry};

use crate::{
    MapInfo, apply_overlay_land_to_pass_grid, pass_grid::PassGrid, seal_pass_grid_from_tmp,
};

/// [`finalize_battle_pass_grid`] 的统计。
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct BattlePassFinalizeStats {
    /// TMP 新封死的格数。
    pub tmp_sealed: usize,
    /// Overlay land 改变通行状态的格数。
    pub overlay_land: usize,
}

/// 对局通行最终封口：已有 Foundation 种子的 [`PassGrid`] → TMP 封格 → overlay land 重开桥面。
///
/// 顺序固定。遭遇战 / 战役 / 测试必须共用，禁止各自重排或跳过回写。
///
/// 调用方在返回后应把 `grid` 同步回 [`ra_types::PreparedMap`] 通行层。
pub fn finalize_battle_pass_grid(
    source: &dyn AssetSource,
    map: &MapInfo,
    overlays: &OverlayTypeRegistry,
    grid: &mut PassGrid,
) -> BattlePassFinalizeStats {
    // 不可把 overlay land 提前到 TMP 之前，否则水格上的桥面会先被重开再被 TMP 封死。
    let tmp_sealed = seal_pass_grid_from_tmp(source, map, grid);
    let overlay_land = apply_overlay_land_to_pass_grid(map, overlays, grid);
    BattlePassFinalizeStats { tmp_sealed, overlay_land }
}
