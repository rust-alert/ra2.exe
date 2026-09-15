//! 对局页控制器：移动单位烤图姿态。

use ra_engine::CELL_MOVE_COST;
use ra_map::MobilePaintPose;
use ra_types::EntityId;

pub use ra_map::slide_offset_along_path;

/// 由权威移动状态计算步兵/载具烤图姿态（含格内滑移偏移）。
///
/// `tick_fraction` 为距下一逻辑 tick 的进度，用于在渲染帧之间继续滑移，避免整格瞬移。
/// 步兵 `sub_cell` 脚点由 [`ra_map::paint_map_mobiles`] 按 `MapEntity.sub_cell` 叠加，不在此重复。
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
