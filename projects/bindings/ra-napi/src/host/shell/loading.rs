//! 进战斗前装载任务（遭遇战 / 战役共用 `LoadScreen`，由 `LoadKind` 区分）。

use std::time::Instant;

use ra_widgets::load_kind::LoadKind;
use ra_widgets::original_screen::OriginalScreen;
use ra_widgets::skirmish_setup::SkirmishBootRequest;

use crate::host::boot::BootResult;
use crate::host::load_job::LoadJob;
use crate::host::battle_controller::BattleController;

use super::Shell;

impl Shell {
    pub(super) fn load_allow_retry(&self) -> bool {
        self.load_job.is_none() && self.pending_load_boot.is_none()
    }

    /// 装载页进度：进行中读任务 ratio；最短展示等待或失败后视为满格。
    pub(super) fn load_screen_progress(&self) -> f32 {
        if self.pending_load_boot.is_some() {
            return 1.0;
        }
        if let Some(job) = self.load_job.as_ref() {
            return job.progress().ratio.clamp(0.0, 1.0);
        }
        if self.screen == OriginalScreen::LoadScreen && self.load_allow_retry() {
            return 1.0;
        }
        0.0
    }

    /// 取消 / 失败提示里的回退去向文案。
    fn load_cancel_hint(&self) -> &'static str {
        match self.load_kind {
            LoadKind::Skirmish => "回大厅",
            LoadKind::Campaign => "回选边",
        }
    }

    /// 开始遭遇战装载（国家 `ls*` 装载图 + 安装目录 boot）。
    pub(super) fn begin_skirmish_load(&mut self) {
        if self.load_job.is_some() || self.pending_load_boot.is_some() {
            tracing::warn!("装载已在进行，忽略重复开始");
            return;
        }
        self.load_kind = LoadKind::Skirmish;
        self.ensure_lobby_maps();
        self.banner =
            format!("正在装载 {} · {}/{}…", self.selected_map.as_deref().unwrap_or("默认候选图"), self.skirmish.side, self.skirmish.difficulty);
        self.pending_after_load = Some(OriginalScreen::Battle);
        self.pending_load_boot = None;
        self.set_screen(OriginalScreen::LoadScreen);
        self.load_started = Some(Instant::now());
        #[cfg(feature = "test-harness")]
        {
            if let Some(scene) = self.test_scene.clone() {
                self.load_job = Some(LoadJob::start_test_scene(scene));
                self.refresh_menu_backdrop();
                self.refresh_shell_title();
                return;
            }
        }
        self.load_job = Some(LoadJob::start_install_boot({
            let mut req = self.skirmish.clone();
            req.preferred_map = self.selected_map.clone();
            req
        }));
        // load_job 赋值后刷新：禁用重试并改标题提示。
        self.refresh_menu_backdrop();
        self.refresh_shell_title();
    }

    /// 按当前 `LoadKind` 重试；战役开局尚未接线时只提示。
    pub(super) fn retry_load(&mut self) {
        if self.load_job.is_some() {
            tracing::info!("装载进行中，忽略重试");
            return;
        }
        match self.load_kind {
            LoadKind::Skirmish => self.begin_skirmish_load(),
            LoadKind::Campaign => {
                self.banner = "战役开局尚未接线 · Esc 回选边".into();
                self.refresh_menu_backdrop();
                self.refresh_shell_title();
            }
        }
    }

    /// 放弃进行中的装载并回到 `LoadKind::cancel_screen`（工作线程结果会被丢弃）。
    pub(super) fn cancel_load(&mut self) {
        if self.load_job.is_none() && self.pending_load_boot.is_none() && self.screen != OriginalScreen::LoadScreen {
            return;
        }
        let back = self.load_kind.cancel_screen();
        self.load_job = None;
        self.pending_load_boot = None;
        self.load_started = None;
        self.pending_after_load = None;
        self.banner = "已取消装载".into();
        tracing::info!(kind = self.load_kind.as_str(), back = back.as_str(), "用户取消装载");
        self.set_screen(back);
    }

    pub(super) fn poll_load_job(&mut self) {
        if let Some(job) = self.load_job.as_ref() {
            match job.try_take() {
                Ok(Some(boot)) => {
                    self.load_job = None;
                    self.pending_load_boot = Some(boot);
                }
                Ok(None) => {}
                Err(()) => {
                    self.load_job = None;
                    self.pending_load_boot = None;
                    self.load_started = None;
                    self.pending_after_load = None;
                    let hint = self.load_cancel_hint();
                    self.banner = format!("装载线程异常断开 · Enter/点重试 · Esc {hint}");
                    tracing::error!(kind = self.load_kind.as_str(), "装载线程异常断开");
                    self.set_screen(OriginalScreen::LoadScreen);
                    self.refresh_menu_backdrop();
                    self.refresh_shell_title();
                    return;
                }
            }
        }

        if self.pending_load_boot.is_some() {
            let min = std::time::Duration::from_secs_f64(self.load_min_secs.max(0.0));
            let ready = self.load_started.map(|t0| t0.elapsed() >= min).unwrap_or(true);
            if ready {
                let boot = self.pending_load_boot.take().expect("pending_load_boot");
                self.load_started = None;
                self.finish_load(boot);
                return;
            }
        }

        if self.load_job.is_some() || self.pending_load_boot.is_some() {
            if let Some(t0) = self.load_started {
                let secs = t0.elapsed().as_secs();
                let pulse = match secs % 3 {
                    0 => ".",
                    1 => "..",
                    _ => "...",
                };
                let stage = if self.pending_load_boot.is_some() {
                    "装载完成，准备进入".into()
                } else {
                    self.load_job.as_ref().map(|job| job.progress().stage).unwrap_or_else(|| "装载中".into())
                };
                let pct = (self.load_screen_progress() * 100.0).round() as i32;
                let next = format!("{stage}{pulse} · {secs}s · {pct}% · Esc 取消");
                if next != self.banner {
                    self.banner = next;
                    self.refresh_menu_backdrop();
                    self.refresh_shell_title();
                }
            }
        }
    }

    pub(super) fn finish_load(&mut self, boot: BootResult) {
        self.banner = boot.note.clone();
        if let Some(preview) = &boot.preview {
            self.renderer.set_map_preview(preview.clone());
        }
        match self.battle_controller.as_mut() {
            Some(ctrl) => ctrl.apply_boot(boot, &mut self.renderer),
            None => self.battle_controller = Some(BattleController::from_boot(boot, self.status_path.clone(), self.test_scene.clone())),
        }
        let ok = self.battle_controller.as_ref().is_some_and(|c| c.has_session());
        let target = self.pending_after_load.take().unwrap_or(OriginalScreen::Battle);
        if ok {
            self.set_screen(target);
        } else {
            let hint = self.load_cancel_hint();
            self.banner = format!("装载失败 · {} · Enter/点重试 · Esc {hint}", self.banner);
            tracing::warn!(kind = self.load_kind.as_str(), "装载失败，停留加载页待重试");
            self.set_screen(OriginalScreen::LoadScreen);
            self.refresh_menu_backdrop();
            self.refresh_shell_title();
        }
    }
}
