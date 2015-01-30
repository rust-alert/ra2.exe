//! 仅 `test-harness` feature：固定合成场景启动（无安装目录）。

use std::path::PathBuf;

use ra_engine::{Engine, Session};
use ra_renderer::RgbaImage;
use ra_testing::standard_duel;
use ra_types::{RaError, RaResult};

/// 测试窗口逻辑尺寸（GUI 自动化基线用）。
pub const TEST_WINDOW_WIDTH: f64 = 1280.0;
pub const TEST_WINDOW_HEIGHT: f64 = 720.0;

/// 解析测试场景名：`--test-scene=` 优先，其次环境变量 `RA2_TEST_SCENE`。
pub fn requested_scene() -> Option<String> {
    for arg in std::env::args().skip(1) {
        if let Some(rest) = arg.strip_prefix("--test-scene=") {
            let s = rest.trim();
            if !s.is_empty() {
                return Some(s.to_ascii_lowercase());
            }
        }
    }
    std::env::var("RA2_TEST_SCENE").ok().map(|s| s.trim().to_ascii_lowercase()).filter(|s| !s.is_empty())
}

/// 可选状态旁路文件路径（`RA2_TEST_STATUS_PATH`）。
pub fn status_path() -> Option<PathBuf> {
    std::env::var_os("RA2_TEST_STATUS_PATH").map(PathBuf::from)
}

pub struct TestBoot {
    pub note: String,
    pub engine: Engine,
    pub session: Session,
    pub preview: Option<RgbaImage>,
}

/// 按场景名装载合成会话。
pub fn boot_scene(scene: &str) -> RaResult<TestBoot> {
    match scene {
        "duel" => boot_duel(),
        other => Err(RaError::Msg(format!("未知测试场景 `{other}`（当前支持: duel）"))),
    }
}

fn boot_duel() -> RaResult<TestBoot> {
    let mut case = standard_duel();
    // 预览原点使等距坐标落入正半幅画布，便于点选与标记对齐。
    case.session.expect_game_mut().set_preview_origin(-240, -40);
    let preview = solid_preview(960, 720, [24, 32, 48, 255]).ok_or_else(|| RaError::Msg("测试预览图分配失败".into()))?;
    Ok(TestBoot { note: "test-harness · scene=duel · synthetic".into(), engine: case.engine, session: case.session, preview: Some(preview) })
}

fn solid_preview(width: u32, height: u32, rgba: [u8; 4]) -> Option<RgbaImage> {
    let n = (width as usize).checked_mul(height as usize)?.checked_mul(4)?;
    let mut pixels = Vec::with_capacity(n);
    for _ in 0..(width * height) {
        pixels.extend_from_slice(&rgba);
    }
    RgbaImage::from_raw(width, height, pixels)
}

/// 写出机器可读会话旁路（给 GUI 自动化轮询）。
pub fn write_status(path: &std::path::Path, session: &Session, selected: &[ra_types::EntityId], screen: &str, leave_armed: bool) {
    let game = session.expect_game();
    let snap = game.snapshot(selected);
    let outcome = match &snap.outcome {
        Some(ra_engine::BattleOutcome::Victory { owner }) => format!("victory:{owner}"),
        None => "none".into(),
    };
    let selected_s = selected.iter().map(|id| id.0.to_string()).collect::<Vec<_>>().join(",");
    let local =
        game.world.players.iter().find(|p| p.id == game.world.local_player).and_then(|lp| snap.players.iter().find(|p| p.house == lp.house));
    let (funds, power_output, power_drain, low_power) =
        local.map(|p| (p.funds, p.power_output, p.power_drain, p.low_power)).unwrap_or((0, 0, 0, false));
    let queue = snap.produce_queues.first().map(|q| format!("{}:{}", q.type_id, q.remaining_ticks)).unwrap_or_else(|| "none".into());
    let last_reject = snap.last_rejects.first().map(|r| format!("{:?}", r.reason)).unwrap_or_else(|| "none".into());
    let body = format!(
        "tick={}\nhash={:#x}\noutcome={}\npaused={}\nselected={}\nentities={}\nfunds={}\npower_output={}\npower_drain={}\nlow_power={}\nqueue={}\nlast_reject={}\ndifficulty={}\nscreen={}\nleave_armed={}\n",
        snap.tick,
        snap.state_hash,
        outcome,
        game.paused,
        selected_s,
        game.world.entity_count(),
        funds,
        power_output,
        power_drain,
        low_power,
        queue,
        last_reject,
        game.difficulty,
        screen,
        leave_armed
    );
    let _ = std::fs::write(path, body);
}
