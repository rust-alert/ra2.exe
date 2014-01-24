//! 关键页验收截图：GPU 回读 / CPU 布局导出 PNG（默认不进 git）。
//!
//! - `F12`：截当前页（GPU 回读）
//! - `RA2_AUTO_SCREENSHOT=1`：进入关键页时各截一次
//! - `RA2_SCREENSHOT_DIR`：输出根目录（默认 `./screenshots`）
//! - `cargo test -p ra-desktop dump_key_ui_screenshots`：自动导出关键页占位图

use std::{
    collections::HashSet,
    path::{Path, PathBuf},
    sync::Arc,
    time::{SystemTime, UNIX_EPOCH},
};

use ra_engine::{HudSnapshot, MatchOutcome, SnapshotPlayer, SnapshotProduceQueue};
use ra_map::Theater;
use ra_renderer::{RgbaImage, ScreenChromeQuad, write_png_file};
use ra_types::{EntityId, RaResult};

use crate::{
    boot::BootMapCandidate,
    hud_chrome::{self, ResultsHit},
    menu_view::{layout_for, layout_skirmish_lobby},
    screen::OriginalScreen,
    ui_assets::stamp_norm_progress_bar,
};

/// 截图输出根目录。
pub fn screenshot_dir() -> PathBuf {
    std::env::var_os("RA2_SCREENSHOT_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("screenshots"))
}

/// 自动测试验收图目录（稳定文件名，便于打开对照）。
pub fn acceptance_dir() -> PathBuf {
    screenshot_dir().join("acceptance")
}

/// 是否在进入关键页时自动截图。
pub fn auto_screenshot_enabled() -> bool {
    matches!(
        std::env::var("RA2_AUTO_SCREENSHOT").as_deref(),
        Ok("1") | Ok("true") | Ok("TRUE") | Ok("yes") | Ok("YES")
    )
}

/// 建议自动截图的关键产品页。
pub fn is_key_screen(screen: OriginalScreen) -> bool {
    matches!(
        screen,
        OriginalScreen::MainMenu
            | OriginalScreen::SinglePlayerMenu
            | OriginalScreen::SkirmishLobby
            | OriginalScreen::LoadScreen
            | OriginalScreen::Match
            | OriginalScreen::Results
            | OriginalScreen::Options
            | OriginalScreen::Network
    )
}

/// 已自动截过的页面（每进程每页一次）。
#[derive(Debug, Default)]
pub struct AutoScreenshotTracker {
    done: HashSet<&'static str>,
}

impl AutoScreenshotTracker {
    /// 若本页尚未自动截过且属于关键页，则标记并返回 `true`。
    pub fn should_capture(&mut self, screen: OriginalScreen) -> bool {
        self.should_capture_if(screen, auto_screenshot_enabled())
    }

    /// 测试 / 显式开关入口。
    pub fn should_capture_if(&mut self, screen: OriginalScreen, enabled: bool) -> bool {
        if !enabled || !is_key_screen(screen) {
            return false;
        }
        let id = screen.as_str();
        if self.done.contains(id) {
            return false;
        }
        self.done.insert(id);
        true
    }
}

/// 将 RGBA 写入 `screenshots/{screen}_{unix_ms}.png`。
pub fn save_screenshot(screen: &str, image: &RgbaImage) -> RaResult<PathBuf> {
    save_screenshot_to(screenshot_dir(), screen, image)
}

/// 指定目录落盘（带时间戳）。
pub fn save_screenshot_to(dir: impl AsRef<Path>, screen: &str, image: &RgbaImage) -> RaResult<PathBuf> {
    let ms = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis())
        .unwrap_or(0);
    let safe = sanitize_name(screen);
    let path = dir.as_ref().join(format!("{safe}_{ms}.png"));
    write_png_file(&path, image)?;
    Ok(path)
}

/// 验收用稳定文件名：`{dir}/{screen}.png`（覆盖写）。
pub fn save_acceptance_png(dir: impl AsRef<Path>, screen: &str, image: &RgbaImage) -> RaResult<PathBuf> {
    let path = dir.as_ref().join(format!("{}.png", sanitize_name(screen)));
    write_png_file(&path, image)?;
    Ok(path)
}

fn sanitize_name(screen: &str) -> String {
    screen
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || c == '_' || c == '-' {
                c
            }
            else {
                '_'
            }
        })
        .collect()
}

/// 仅用于单元测试的路径拼接（不写盘）。
pub fn planned_path(dir: &Path, screen: &str, ms: u128) -> PathBuf {
    dir.join(format!("{screen}_{ms}.png"))
}

/// 把归一化色块叠到 RGBA 底图上（CPU，供无 GPU 的验收导出）。
pub fn paint_screen_chrome(dst: &mut RgbaImage, quads: &[ScreenChromeQuad]) {
    let w = dst.width.max(1) as f32;
    let h = dst.height.max(1) as f32;
    for q in quads {
        let x0 = (q.x0.clamp(0.0, 1.0) * w) as u32;
        let y0 = (q.y0.clamp(0.0, 1.0) * h) as u32;
        let x1 = (q.x1.clamp(0.0, 1.0) * w) as u32;
        let y1 = (q.y1.clamp(0.0, 1.0) * h) as u32;
        let src = [
            (q.color[0].clamp(0.0, 1.0) * 255.0) as u8,
            (q.color[1].clamp(0.0, 1.0) * 255.0) as u8,
            (q.color[2].clamp(0.0, 1.0) * 255.0) as u8,
            (q.color[3].clamp(0.0, 1.0) * 255.0) as u8,
        ];
        blend_rect(dst, x0, y0, x1.saturating_sub(x0).max(1), y1.saturating_sub(y0).max(1), src);
    }
}

fn blend_rect(dst: &mut RgbaImage, x: u32, y: u32, bw: u32, bh: u32, src: [u8; 4]) {
    let x1 = (x + bw).min(dst.width);
    let y1 = (y + bh).min(dst.height);
    let sa = src[3] as u32;
    if sa == 0 {
        return;
    }
    for py in y..y1 {
        for px in x..x1 {
            let i = ((py * dst.width + px) * 4) as usize;
            if sa >= 255 {
                dst.pixels[i..i + 4].copy_from_slice(&src);
                continue;
            }
            let inv = 255 - sa;
            for c in 0..3 {
                let d = dst.pixels[i + c] as u32;
                dst.pixels[i + c] = ((src[c] as u32 * sa + d * inv) / 255) as u8;
            }
            dst.pixels[i + 3] = 255;
        }
    }
}

fn solid_bg(width: u32, height: u32, rgba: [u8; 4]) -> RgbaImage {
    let mut pixels = vec![0u8; (width as usize) * (height as usize) * 4];
    for px in pixels.chunks_exact_mut(4) {
        px.copy_from_slice(&rgba);
    }
    RgbaImage::new(width, height, pixels).expect("solid bg")
}

fn sample_hud() -> HudSnapshot {
    HudSnapshot {
        tick: 240,
        players: vec![SnapshotPlayer {
            house: Arc::from("Americans"),
            funds: 4200,
            power_output: 100,
            power_drain: 60,
            low_power: false,
        }],
        produce_queues: vec![SnapshotProduceQueue {
            factory: EntityId(7),
            type_id: Arc::from("E1"),
            remaining_ticks: 8,
            rally_x: None,
            rally_y: None,
        }],
        last_rejects: vec![],
        outcome: None,
        paused: false,
        pause_reason: None,
        match_stats: None,
    }
}

/// 导出全部关键占位页到 `screenshots/acceptance/*.png`（覆盖写）。
///
/// **无窗口 / 无 GPU**：菜单走 `menu_view` 色块布局；对局/结算把 HUD chrome 叠在合成底图上。
/// 这是验收对照图，不是原版 SHP 截图。
pub fn dump_all_key_screens() -> RaResult<Vec<PathBuf>> {
    dump_all_key_screens_to(acceptance_dir())
}

/// 导出到指定目录。
pub fn dump_all_key_screens_to(dir: impl AsRef<Path>) -> RaResult<Vec<PathBuf>> {
    let dir = dir.as_ref();
    let (w, h) = (1280u32, 720u32);
    let mut out = Vec::new();

    let menu_pages = [
        OriginalScreen::MainMenu,
        OriginalScreen::SinglePlayerMenu,
        OriginalScreen::LoadScreen,
        OriginalScreen::Options,
        OriginalScreen::Network,
    ];
    for screen in menu_pages {
        let mut layout = layout_for(screen, w, h, None, None).expect("menu layout");
        if screen == OriginalScreen::LoadScreen {
            // 与壳层装载进度条一致：落在 loading 槽位内。
            stamp_norm_progress_bar(
                &mut layout.image,
                0.32,
                0.42,
                0.72,
                0.46,
                0.45,
                [28, 32, 48, 255],
                [220, 180, 64, 255],
            );
        }
        out.push(save_acceptance_png(dir, screen.as_str(), &layout.image)?);
    }

    // 主菜单悬停态：第一个可点入口（single_player）。
    let main_hover = layout_for(OriginalScreen::MainMenu, w, h, Some(0), None).expect("menu hover");
    out.push(save_acceptance_png(dir, "main_menu_hover", &main_hover.image)?);

    let maps = [
        BootMapCandidate {
            file_name: "mp03t4.map".into(),
            width: 100,
            height: 90,
            theater: Theater::Temperate,
        },
        BootMapCandidate {
            file_name: "sample_snow.map".into(),
            width: 80,
            height: 80,
            theater: Theater::Snow,
        },
    ];
    let lobby = layout_skirmish_lobby(
        w,
        h,
        &maps,
        Some("mp03t4.map"),
        "Americans",
        "Normal",
        Some(0),
        None,
    );
    out.push(save_acceptance_png(dir, OriginalScreen::SkirmishLobby.as_str(), &lobby.image)?);

    let lobby_alt = layout_skirmish_lobby(
        w,
        h,
        &maps,
        Some("sample_snow.map"),
        "Russians",
        "Hard",
        None,
        None,
    );
    out.push(save_acceptance_png(dir, "skirmish_lobby_alt", &lobby_alt.image)?);

    let mut match_img = solid_bg(w, h, [24, 48, 28, 255]);
    let hud = sample_hud();
    paint_screen_chrome(&mut match_img, &hud_chrome::match_hud_chrome(&hud, Some("Americans")));
    out.push(save_acceptance_png(dir, OriginalScreen::Match.as_str(), &match_img)?);

    let mut match_paused = solid_bg(w, h, [24, 48, 28, 255]);
    let mut paused_hud = sample_hud();
    paused_hud.paused = true;
    paused_hud.pause_reason = Some("验收暂停".into());
    paint_screen_chrome(
        &mut match_paused,
        &hud_chrome::match_hud_chrome(&paused_hud, Some("Americans")),
    );
    out.push(save_acceptance_png(dir, "match_paused", &match_paused)?);

    let mut match_reject = solid_bg(w, h, [24, 48, 28, 255]);
    let mut reject_hud = sample_hud();
    reject_hud.last_rejects.push(ra_engine::CommandReject {
        command_index: 0,
        reason: ra_engine::CommandRejectReason::InsufficientFunds,
    });
    paint_screen_chrome(
        &mut match_reject,
        &hud_chrome::match_hud_chrome(&reject_hud, Some("Americans")),
    );
    out.push(save_acceptance_png(dir, "match_reject", &match_reject)?);

    let mut results_img = solid_bg(w, h, [20, 28, 40, 255]);
    let mut results_hud = sample_hud();
    results_hud.outcome = Some(MatchOutcome::Victory {
        owner: "Americans".into(),
    });
    paint_screen_chrome(
        &mut results_img,
        &hud_chrome::match_hud_chrome(&results_hud, Some("Americans")),
    );
    paint_screen_chrome(&mut results_img, &hud_chrome::results_chrome(Some(ResultsHit::Rematch)));
    out.push(save_acceptance_png(dir, OriginalScreen::Results.as_str(), &results_img)?);

    let mut results_lobby = solid_bg(w, h, [20, 28, 40, 255]);
    paint_screen_chrome(
        &mut results_lobby,
        &hud_chrome::match_hud_chrome(&results_hud, Some("Americans")),
    );
    paint_screen_chrome(
        &mut results_lobby,
        &hud_chrome::results_chrome(Some(ResultsHit::ToLobby)),
    );
    out.push(save_acceptance_png(dir, "results_lobby_hover", &results_lobby)?);

    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn key_screens_include_main_menu_and_results() {
        assert!(is_key_screen(OriginalScreen::MainMenu));
        assert!(is_key_screen(OriginalScreen::Results));
    }

    #[test]
    fn planned_path_uses_screen_and_millis() {
        let p = planned_path(Path::new("screenshots"), "main_menu", 42);
        assert_eq!(p, PathBuf::from("screenshots/main_menu_42.png"));
    }

    #[test]
    fn tracker_fires_once_per_screen() {
        let mut t = AutoScreenshotTracker::default();
        assert!(t.should_capture_if(OriginalScreen::MainMenu, true));
        assert!(!t.should_capture_if(OriginalScreen::MainMenu, true));
        assert!(!t.should_capture_if(OriginalScreen::MainMenu, false));
    }

    #[test]
    fn dump_key_ui_screenshots_for_acceptance() {
        let paths = dump_all_key_screens().expect("dump key screens");
        let dir = acceptance_dir();
        assert!(paths.len() >= 7, "expected key screens, got {}", paths.len());
        for path in &paths {
            assert!(path.exists(), "missing {}", path.display());
            let len = std::fs::metadata(path).unwrap().len();
            assert!(len > 200, "{} too small ({len})", path.display());
        }
        assert!(dir.join("main_menu.png").exists());
        assert!(dir.join("main_menu_hover.png").exists());
        assert!(dir.join("skirmish_lobby.png").exists());
        assert!(dir.join("skirmish_lobby_alt.png").exists());
        assert!(dir.join("match.png").exists());
        assert!(dir.join("match_paused.png").exists());
        assert!(dir.join("match_reject.png").exists());
        assert!(dir.join("results.png").exists());
        assert!(dir.join("results_lobby_hover.png").exists());
    }

    #[test]
    fn paint_chrome_covers_pixels() {
        let mut img = solid_bg(100, 100, [0, 0, 0, 255]);
        paint_screen_chrome(
            &mut img,
            &[ScreenChromeQuad {
                x0: 0.0,
                y0: 0.0,
                x1: 0.5,
                y1: 0.5,
                color: [1.0, 0.0, 0.0, 1.0],
            }],
        );
        assert_eq!(img.pixels[0..3], [255, 0, 0]);
    }
}
