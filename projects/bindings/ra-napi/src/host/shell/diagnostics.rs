//! 截图与状态栏诊断。

use ra_widgets::original_screen::OriginalScreen;
use ra_widgets::skin::text::{
    campaign_csf_tooltip, main_menu_csf_tooltip, resolve_csf_text, single_player_csf_tooltip,
    skirmish_lobby_csf_tooltip,
};

use crate::host::screenshot;

use super::Shell;

impl Shell {
    /// 请求下一帧 GPU 回读并落盘为 `{name}_*.png`。
    pub(super) fn queue_screenshot(&mut self, name: &'static str) {
        self.renderer.request_capture();
        self.pending_screenshot = Some(name);
    }

    pub(super) fn flush_pending_screenshot(&mut self) {
        let Some(name) = self.pending_screenshot.take()
        else {
            return;
        };
        if let Some(err) = self.renderer.take_capture_error() {
            tracing::error!("截图回读失败 · screen={name} · {err}");
            self.banner = format!("截图失败 · {err}");
            return;
        }
        let Some(image) = self.renderer.take_capture()
        else {
            tracing::warn!("截图尚未就绪 · screen={name}");
            return;
        };
        match screenshot::save_screenshot(name, &image) {
            Ok(path) => {
                tracing::info!(%name, path = %path.display(), "关键页截图已保存");
                self.banner = format!("截图已保存 · {}", path.display());
            }
            Err(e) => tracing::error!("截图保存失败 · {e}"),
        }
    }

    /// 当前底栏可见切片；空串或切页进出/卡顿中视为无提示。
    pub(super) fn status_line_visible(&self) -> Option<&str> {
        if self.shell_slide_busy() {
            return None;
        }
        let text = self.status_line.visible();
        if text.is_empty() { None } else { Some(text) }
    }

    /// 按当前页面与悬停入口解析 CSF 提示，提交给打字机。
    pub(super) fn sync_status_line_from_hover(&mut self) {
        let Some(entry) = self.menu_hovered_entry
        else {
            self.status_line.clear();
            return;
        };
        let key = match self.screen {
            OriginalScreen::MainMenu => main_menu_csf_tooltip(entry),
            OriginalScreen::SinglePlayerMenu => single_player_csf_tooltip(entry),
            OriginalScreen::Campaign => campaign_csf_tooltip(entry),
            OriginalScreen::SkirmishLobby => skirmish_lobby_csf_tooltip(entry),
            _ => None,
        };
        let text = key.and_then(|k| resolve_csf_text(self.menu_csf.as_ref(), k)).unwrap_or_default();
        if text.is_empty() {
            self.status_line.clear();
        }
        else {
            self.status_line.set_text(text);
        }
    }
}
