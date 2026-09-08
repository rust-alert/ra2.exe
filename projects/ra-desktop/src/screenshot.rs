//! 关键页验收截图：GPU 回读落盘 PNG（默认不进 git）。
//!
//! - `F12`：截当前页（GPU 回读）
//! - `RA2_AUTO_SCREENSHOT=1`：进入关键页时各截一次
//! - `RA2_SCREENSHOT_DIR`：输出根目录（默认 `./screenshots`）
//!
//! 不提供色块菜单 / 假 HUD 的 CPU 导出。真实 UI 接线后再做验收截图基线。

use std::{
    collections::HashSet,
    path::{Path, PathBuf},
    time::{SystemTime, UNIX_EPOCH},
};

use ra_renderer::{RgbaImage, write_png_file};
use ra_types::RaResult;

use crate::screen::OriginalScreen;

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
}
