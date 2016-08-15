//! 覆盖层 `Land=` / `NoUseTileLandType`：在 TMP 封格后重开桥面等可走覆盖层。

use ra_types::OverlayTypeRegistry;

use crate::{MapInfo, pass_grid::PassGrid};

/// 对已冻结的 `NoUseTileLandType` 覆盖按装载期通行结果重写格子。
///
/// 返回新打开或新封死的格数（状态变化数）。
pub fn apply_overlay_land_to_pass_grid(map: &MapInfo, overlays: &OverlayTypeRegistry, grid: &mut PassGrid) -> usize {
    let mut changed = 0usize;
    for cell in &map.overlays {
        let Some(passable) = overlays.land_pass_override(cell.overlay_id)
        else {
            continue;
        };
        if grid.is_passable(cell.x, cell.y) == passable {
            continue;
        }
        grid.set_passable(cell.x, cell.y, passable);
        changed += 1;
    }
    changed
}
