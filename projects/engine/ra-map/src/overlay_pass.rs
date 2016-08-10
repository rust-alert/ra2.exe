//! 覆盖层 `Land=` / `NoUseTileLandType`：在 TMP 封格后重开桥面等可走覆盖层。

use ra_assets::IniDocument;

use crate::{
    MapInfo,
    land::{LandType, land_passable},
    pass_grid::PassGrid,
};

/// 对 `NoUseTileLandType=yes` 的覆盖层按 `Land=` 重写通行（典型：低桥 `LOBRDG*` → `Road`）。
///
/// `overlay_type_name`：`overlay_id` → rules 类型名（大写）。返回新打开或新封死的格数（状态变化数）。
pub fn apply_overlay_land_to_pass_grid(
    map: &MapInfo,
    rules: &IniDocument,
    overlay_type_name: &dyn Fn(u8) -> Option<String>,
    grid: &mut PassGrid,
) -> usize {
    let mut changed = 0usize;
    for cell in &map.overlays {
        let Some(name) = overlay_type_name(cell.overlay_id)
        else {
            continue;
        };
        let no_use = rules.get(&name, "NoUseTileLandType").is_some_and(|v| v.eq_ignore_ascii_case("yes") || v == "1");
        if !no_use {
            continue;
        }
        let land = rules.get(&name, "Land").and_then(LandType::parse_name).unwrap_or(LandType::Clear);
        let passable = land_passable(land);
        if grid.is_passable(cell.x, cell.y) == passable {
            continue;
        }
        grid.set_passable(cell.x, cell.y, passable);
        changed += 1;
    }
    changed
}
