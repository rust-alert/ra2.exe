//! 对局页控制器：输入意图、命令、tick、快照；不含窗口与页面导航外壳。

use ra_engine::CELL_MOVE_COST;
use ra_map::{MobilePaintPose, iso_to_screen};
use ra_types::EntityId;

/// 由权威移动状态计算步兵/载具烤图姿态（含格内滑移偏移）。
///
/// `tick_fraction` 为距下一逻辑 tick 的进度，用于在渲染帧之间继续滑移，避免整格瞬移。
pub fn mobile_paint_pose_for(game: &ra_engine::BattleSession, id: EntityId, cell_x: u16, cell_y: u16, tick_fraction: f64) -> MobilePaintPose {
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
    }
    else {
        (0, 0)
    };
    MobilePaintPose { anim_frame, moving, offset_x, offset_y }
}

/// 沿路径用 `move_accum + speed * tick_fraction` 计算相对当前逻辑格的屏幕像素偏移。
pub fn slide_offset_along_path(
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
