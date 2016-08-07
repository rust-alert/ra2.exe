//! 对局页控制器：输入意图、命令、tick、快照；不含窗口与页面导航外壳。

use ra_engine::CELL_MOVE_COST;
use ra_map::{
    MobilePaintPose, iso_to_screen,
};
use ra_types::EntityId;


/// 由权威移动状态计算步兵/载具烤图姿态（含格内滑移偏移）。
///
/// `tick_fraction` 为距下一逻辑 tick 的进度，用于在渲染帧之间继续滑移，避免整格瞬移。
pub(super) fn mobile_paint_pose_for(game: &ra_engine::BattleSession, id: EntityId, cell_x: u16, cell_y: u16, tick_fraction: f64) -> MobilePaintPose {
    let anim_frame = game.world.ecs_animation(id).map(|(f, _)| f).unwrap_or(0);
    let moving =
        game.world.ecs_move_destination(id).is_some_and(|(dx, _)| dx.is_some()) || game.world.ecs_path(id).is_some_and(|p| !p.is_empty());
    let (offset_x, offset_y) = if moving {
        let accum = game.world.ecs_move_accum(id).unwrap_or(0);
        let speed = game.world.ecs_speed(id).unwrap_or(0);
        let path = game.world.ecs_path(id).unwrap_or_default();
        slide_offset_along_path(cell_x, cell_y, &path, accum, speed, tick_fraction, CELL_MOVE_COST, |x, y| {
            game.world.pass_grid.cell_height(x, y)
        })
    } else {
        (0, 0)
    };
    MobilePaintPose { anim_frame, moving, offset_x, offset_y }
}

/// 沿路径用 `move_accum + speed * tick_fraction` 计算相对当前逻辑格的屏幕像素偏移。
pub(super) fn slide_offset_along_path(
    cell_x: u16,
    cell_y: u16,
    path: &[(u16, u16)],
    move_accum: u32,
    speed: u32,
    tick_fraction: f64,
    cell_cost: u32,
    cell_z: impl Fn(u16, u16) -> u8,
) -> (i32, i32) {
    if cell_cost == 0 || path.is_empty() {
        return (0, 0);
    }
    let cost = cell_cost as f32;
    let mut visual = move_accum as f32 + speed as f32 * (tick_fraction as f32).clamp(0.0, 1.0);
    let mut from_x = cell_x;
    let mut from_y = cell_y;
    let mut path_i = 0usize;
    // 预测跨越的整格：逻辑格仍停在 from，呈现滑到后续路点。
    while visual >= cost && path_i + 1 < path.len() {
        visual -= cost;
        let (nx, ny) = path[path_i];
        from_x = nx;
        from_y = ny;
        path_i += 1;
    }
    let Some(&(nx, ny)) = path.get(path_i)
    else {
        return (0, 0);
    };
    let t = (visual / cost).clamp(0.0, 1.0);
    let z0 = cell_z(from_x, from_y);
    let z1 = cell_z(nx, ny);
    let (sx0, sy0) = iso_to_screen(i32::from(from_x), i32::from(from_y), z0);
    let (sx1, sy1) = iso_to_screen(i32::from(nx), i32::from(ny), z1);
    // 偏移相对实体逻辑格（cell_x/y）的屏幕原点，而非预测 from。
    let z_logic = cell_z(cell_x, cell_y);
    let (sx_logic, sy_logic) = iso_to_screen(i32::from(cell_x), i32::from(cell_y), z_logic);
    let sx = sx0 as f32 + (sx1 - sx0) as f32 * t;
    let sy = sy0 as f32 + (sy1 - sy0) as f32 * t;
    ((sx - sx_logic as f32).round() as i32, (sy - sy_logic as f32).round() as i32)
}

#[cfg(test)]
mod slide_offset_tests {
    use super::slide_offset_along_path;

    #[test]
    pub(super) fn slide_offset_moves_toward_next_cell() {
        // 等距邻格：进度一半时应有明显非零偏移（非整格瞬移）。
        let path = [(2u16, 1u16)];
        let (ox0, oy0) = slide_offset_along_path(1, 1, &path, 0, 0, 0.0, 64, |_, _| 0);
        assert_eq!((ox0, oy0), (0, 0));
        let (ox, oy) = slide_offset_along_path(1, 1, &path, 32, 0, 0.0, 64, |_, _| 0);
        assert!(ox != 0 || oy != 0, "mid-cell slide must leave cell origin");
        let (ox1, oy1) = slide_offset_along_path(1, 1, &path, 64, 0, 0.0, 64, |_, _| 0);
        let (ox_half, oy_half) = (ox, oy);
        assert!(ox1.abs() >= ox_half.abs() || oy1.abs() >= oy_half.abs());
    }

    #[test]
    pub(super) fn tick_fraction_extends_slide_between_logic_ticks() {
        let path = [(2u16, 1u16)];
        let (a, b) = slide_offset_along_path(1, 1, &path, 0, 32, 0.0, 64, |_, _| 0);
        let (c, d) = slide_offset_along_path(1, 1, &path, 0, 32, 0.5, 64, |_, _| 0);
        assert_eq!((a, b), (0, 0));
        assert!(c != 0 || d != 0, "render fraction must advance slide without waiting for next logic tick");
    }
}
