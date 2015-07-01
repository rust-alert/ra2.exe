//! 启动参数与事件循环入口。

use std::path::PathBuf;

use ra_layout::ui_layout;
use ra_types::{DisplayMode, PresentFeel, RaError, RaResult};
use winit::event_loop::{ControlFlow, EventLoop};

use crate::host::config;

#[cfg(feature = "test-harness")]
use crate::host::boot::BootResult;

use super::Shell;

/// 战役难度轨鼠标 X → 档位 0..=2（与遭遇战滑条同一套整数映射）。
pub fn campaign_difficulty_from_track_x(track: ui_layout::RectPx, mouse_x: i32) -> u8 {
    let travel = (track.w - 12).max(1);
    let rel = (mouse_x - track.x - 6).clamp(0, travel);
    ((rel * 2 + travel / 2) / travel).clamp(0, 2) as u8
}

/// 解析启动参数并进入事件循环。
pub fn run_shell() -> RaResult<()> {
    let (mode, display_mode, music_volume, sound_volume, present, load_min_secs, shell_slide_gap_secs, status_path, test_scene, start_screen) =
        resolve_launch()?;

    let event_loop = EventLoop::new().map_err(|e| RaError::Msg(e.to_string()))?;
    event_loop.set_control_flow(ControlFlow::Poll);

    let mut app = match mode {
        #[cfg(feature = "test-harness")]
        LaunchMode::DirectBattle(boot) => {
            if let Some(game) = boot.session.as_ref().and_then(|s| s.battle()) {
                tracing::info!(
                    "preview_origin=({}, {}) entities={}",
                    game.preview_origin_x,
                    game.preview_origin_y,
                    game.world.entity_count()
                );
            }
            Shell::with_match(
                boot,
                display_mode.size().0 as f64,
                display_mode.size().1 as f64,
                status_path,
                test_scene,
            )
        }
        LaunchMode::MainMenu => {
            let _ = (status_path, test_scene);
            Shell::with_main_menu(display_mode, start_screen)
        }
    };
    app.apply_audio_volumes(music_volume, sound_volume);
    app.apply_present_feel(present);
    app.load_min_secs = load_min_secs;
    app.shell_slide_gap_secs = shell_slide_gap_secs;

    event_loop.run_app(&mut app).map_err(|e| RaError::Msg(e.to_string()))?;
    tracing::info!("事件循环结束");
    Ok(())
}

enum LaunchMode {
    #[cfg(feature = "test-harness")]
    DirectBattle(BootResult),
    MainMenu,
}

fn resolve_launch() -> RaResult<(
    LaunchMode,
    DisplayMode,
    f32,
    f32,
    PresentFeel,
    f64,
    f64,
    Option<PathBuf>,
    Option<String>,
    ra_widgets::original_screen::OriginalScreen,
)> {
    #[cfg(feature = "test-harness")]
    {
        if let Some(scene) = crate::host::test_boot::requested_scene() {
            let status_path = crate::host::test_boot::status_path();
            let window_width = crate::host::test_boot::TEST_WINDOW_WIDTH;
            let window_height = crate::host::test_boot::TEST_WINDOW_HEIGHT;
            tracing::info!(
                "test-harness scene={scene} window={}x{} status={}",
                window_width,
                window_height,
                status_path
                    .as_ref()
                    .map(|p| p.display().to_string())
                    .unwrap_or_else(|| "—".into())
            );
            let t = crate::host::test_boot::boot_scene(&scene)?;
            tracing::info!("boot: {} · session=ok", t.note);
            return Ok((
                LaunchMode::DirectBattle(BootResult {
                    note: t.note,
                    engine: Some(t.engine),
                    session: Some(t.session),
                    preview: t.preview,
                }),
                DisplayMode::DEFAULT,
                0.4,
                0.7,
                PresentFeel::DEFAULT,
                0.0,
                0.0,
                status_path,
                Some(scene),
                ra_widgets::original_screen::OriginalScreen::Battle,
            ));
        }
    }

    // 产品路径：默认可从闪屏起；`--screen` / 配置 `screen` 可直达遭遇战等前置页。
    let (settings, diagnostics) = config::load_desktop_config_with_diagnostics();
    for d in &diagnostics {
        tracing::info!(source = %d.source, "{}", d.message);
    }
    let display_mode = settings.display_mode;
    let start_screen = match settings.screen.as_deref() {
        None => ra_widgets::original_screen::OriginalScreen::Splash,
        Some(raw) => ra_widgets::original_screen::OriginalScreen::parse_launch_alias(raw).map_err(RaError::Msg)?,
    };
    tracing::info!(
        display_mode = display_mode.as_str(),
        start_screen = start_screen.as_str(),
        music_volume = settings.music_volume,
        sound_volume = settings.sound_volume,
        load_min_secs = settings.load_min_secs,
        shell_slide_gap_secs = settings.shell_slide_gap_secs,
        present_mode = settings.present.mode.as_str(),
        ra2_dir = %settings.ra2_dir.display(),
        "desktop launch settings"
    );
    Ok((
        LaunchMode::MainMenu,
        display_mode,
        settings.music_volume,
        settings.sound_volume,
        settings.present,
        settings.load_min_secs,
        settings.shell_slide_gap_secs,
        None,
        None,
        start_screen,
    ))
}
