//! 对局页控制器：输入意图、命令、tick、快照；不含窗口与页面导航外壳。

use ra_engine::CELL_MOVE_COST;
use ra_map::{MobilePaintPose, iso_to_screen};
use ra_types::EntityId;

/// 由权威移动状态计算步兵/载具烤图姿态（含格内滑移偏移）。
///
/// `tick_fraction` 为距下一逻辑 tick 的进度，用于在渲染帧之间继续滑移，避免整格瞬移。
pub fn mobile_paint_pose_for(game: &ra_engine::BattleSession, id: EntityId, cell_x: u16, cell_y: u16, tick_fraction: f64) -> MobilePaintPose {
    let fire_flash = game.world.ecs_fire_flash(id).unwrap_or(0);
    let firing = fire_flash > 0;
    // 开火窗口内用 `fire_flash` 进度驱动步兵 Fire 序列与载具 HVA 后坐，避免沿用行走 `hva_frame`。
    let anim_frame = if firing {
        ra_engine::FIRE_FLASH_TICKS.saturating_sub(fire_flash) as u16
    }
    else {
        game.world.ecs_animation(id).map(|(f, _)| f).unwrap_or(0)
    };
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
    MobilePaintPose {
        anim_frame,
        moving,
        firing,
        hit_flash: game.world.ecs_animation(id).map(|(_, h)| h > 0).unwrap_or(false),
        offset_x,
        offset_y,
        turret_facing: game.world.ecs_turret_facing(id),
    }
}

/// 相对当前逻辑格，沿 `path[0]` 单边插值屏幕偏移。
///
/// 只用「当前格 → 下一格」一条边：`t = (move_accum + speed * tick_fraction) / cell_cost`，
/// 钳到 `[0, 1]`。不跨后续路点预测，避免 repath 时呈现跳格。
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
    let Some(&(nx, ny)) = path.first()
    else {
        return (0, 0);
    };
    let cost = cell_cost as f32;
    let visual = move_accum as f32 + speed as f32 * (tick_fraction as f32).clamp(0.0, 1.0);
    let t = (visual / cost).clamp(0.0, 1.0);
    let z0 = cell_z(cell_x, cell_y);
    let z1 = cell_z(nx, ny);
    let (sx0, sy0) = iso_to_screen(i32::from(cell_x), i32::from(cell_y), z0);
    let (sx1, sy1) = iso_to_screen(i32::from(nx), i32::from(ny), z1);
    let sx = sx0 as f32 + (sx1 - sx0) as f32 * t;
    let sy = sy0 as f32 + (sy1 - sy0) as f32 * t;
    ((sx - sx0 as f32).round() as i32, (sy - sy0 as f32).round() as i32)
}
