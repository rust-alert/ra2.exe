//! 选项页草稿与提交。

use ra_types::{DisplayMode, PresentFeel};
use ra_widgets::options_dialog;
use ra_widgets::original_screen::OriginalScreen;

use super::Shell;

impl Shell {
    /// 应用壳层质感呈现配置（上传 UI 页前生效）。
    pub(super) fn apply_present_feel(&mut self, present: PresentFeel) {
        self.present = present.sanitized();
        tracing::info!(
            mode = self.present.mode.as_str(),
            quantize = self.present.quantize.as_str(),
            dither = self.present.dither,
            "已应用壳层质感呈现"
        );
    }

    /// 用选项草稿中的音乐/音效滑条即时推到设备（不落盘）。
    pub(super) fn sync_options_live_volumes(&mut self) {
        let Some(state) = self.options_state.as_ref()
        else {
            return;
        };
        let music = state.music_volume_f32();
        let sound = state.sound_volume_f32();
        if let Some(audio) = self.audio.as_mut() {
            audio.set_music_volume(music);
            audio.set_sfx_volume(sound);
        }
    }

    /// 用选项草稿中的质感即时推到壳层（不落盘，避免拖动时刷日志）。
    pub(super) fn sync_options_live_present(&mut self) {
        let Some(state) = self.options_state.as_ref()
        else {
            return;
        };
        let next = state.present.sanitized();
        if next != self.present {
            self.present = next;
        }
    }

    /// 选项页按下：左栏优先；右栏仍走原有 pressed 精灵。
    pub(super) fn handle_options_press(&mut self) -> bool {
        let layout = ra_widgets::options_dialog::OptionsDialogLayout::new();
        let (x, y) = self.shell_cursor_px();
        let Some(hit) = self.options_state.as_mut().and_then(|state| state.on_press(&layout, x, y))
        else {
            return false;
        };
        use ra_widgets::options_dialog::OptionsHit;
        match hit {
            OptionsHit::Accept => {
                self.menu_pressed_entry = Some("accept");
                self.play_menu_click();
                self.refresh_menu_backdrop();
                true
            }
            OptionsHit::Cancel => {
                self.menu_pressed_entry = Some("cancel");
                self.play_menu_click();
                self.refresh_menu_backdrop();
                true
            }
            OptionsHit::MainMenu => {
                self.menu_pressed_entry = Some("main_menu");
                self.play_menu_click();
                self.refresh_menu_backdrop();
                true
            }
            OptionsHit::Track(_) | OptionsHit::Toggle(_) | OptionsHit::ResolutionCombo | OptionsHit::ResolutionRow(_) => {
                self.options_pointer_consumed = true;
                self.play_menu_click();
                self.sync_options_live_volumes();
                self.sync_options_live_present();
                self.refresh_menu_backdrop();
                true
            }
        }
    }

    /// 选项页拖动滑条。
    pub(super) fn handle_options_drag(&mut self) -> bool {
        let layout = ra_widgets::options_dialog::OptionsDialogLayout::new();
        let (x, y) = self.shell_cursor_px();
        let dragged = self.options_state.as_mut().map(|state| state.dragging.is_some() && state.on_drag(&layout, x, y)).unwrap_or(false);
        if !dragged {
            return false;
        }
        self.sync_options_live_volumes();
        self.sync_options_live_present();
        self.refresh_menu_backdrop();
        true
    }

    /// 进入选项页并快照当前显示档 / 音量 / 质感草稿。
    pub(super) fn open_options_page(&mut self) {
        let (music, sound) = self.audio.as_ref().map(|a| (a.music_volume(), a.sfx_volume())).unwrap_or((0.4, 0.7));
        self.options_volume_baseline = Some((music, sound));
        self.options_present_baseline = Some(self.present);
        self.options_pointer_consumed = false;
        self.options_state = Some(ra_widgets::options_dialog::OptionsDialogState::from_shell(self.display_mode, music, sound, self.present));
        self.set_screen(OriginalScreen::Options);
    }

    /// 丢弃选项草稿并还原进入页前的音量 / 质感预览。
    pub(super) fn discard_options_draft(&mut self) {
        if let Some((music, sound)) = self.options_volume_baseline.take() {
            if let Some(audio) = self.audio.as_mut() {
                audio.set_music_volume(music);
                audio.set_sfx_volume(sound);
            }
        }
        if let Some(present) = self.options_present_baseline.take() {
            self.present = present.sanitized();
        }
        self.options_state = None;
        self.options_pointer_consumed = false;
    }

    /// 接受选项草稿：音量与质感立刻生效并落盘，分辨率变更则改窗。
    pub(super) fn apply_options_accept(&mut self) {
        let Some(state) = self.options_state.take()
        else {
            self.options_volume_baseline = None;
            self.options_present_baseline = None;
            self.set_screen(OriginalScreen::MainMenu);
            return;
        };
        self.options_volume_baseline = None;
        self.options_present_baseline = None;
        self.options_pointer_consumed = false;
        let music = state.music_volume_f32();
        let sound = state.sound_volume_f32();
        self.apply_audio_volumes(music, sound);
        match ra_config::DesktopSettings::persist_audio_volumes(music, sound) {
            Ok(()) => tracing::info!(music, sound, "已写入壳层音量"),
            Err(e) => tracing::warn!(error = %e, "写入壳层音量失败"),
        }
        self.apply_present_feel(state.present);
        match ra_config::DesktopSettings::persist_present_feel(self.present) {
            Ok(()) => tracing::info!(mode = self.present.mode.as_str(), dither = self.present.dither, "已写入 [present]"),
            Err(e) => tracing::warn!(error = %e, "写入 [present] 失败"),
        }
        if state.display_mode != self.display_mode {
            self.apply_display_mode(state.display_mode);
        }
        self.banner = "选项已保存".into();
        self.set_screen(OriginalScreen::MainMenu);
        self.refresh_shell_title();
    }

    /// 应用指定 `DisplayMode`（改窗、落盘、刷新）。
    pub(super) fn apply_display_mode(&mut self, mode: DisplayMode) {
        self.display_mode = mode;
        let (w, h) = self.display_mode.size();
        self.window_width = w as f64;
        self.window_height = h as f64;
        if let Some(window) = self.window.as_ref() {
            let _ = window.request_inner_size(winit::dpi::LogicalSize::new(self.window_width, self.window_height));
        }
        match ra_config::DesktopSettings::persist_display_mode(self.display_mode) {
            Ok(()) => {
                tracing::info!(display_mode = self.display_mode.as_str(), "已写入 display_mode");
                self.banner = format!("分辨率 · {}", self.display_mode.as_str());
            }
            Err(e) => {
                tracing::warn!(error = %e, "写入 display_mode 失败");
                self.banner = format!("分辨率 · {} · 写入失败", self.display_mode.as_str());
            }
        }
        self.refresh_menu_backdrop();
        self.refresh_shell_title();
    }
}
