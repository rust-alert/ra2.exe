//! 进战斗前装载任务（遭遇战 / 战役共用 `LoadScreen`，由 `LoadKind` 区分）。

use std::time::Instant;

use ra_widgets::{
    campaign_setup::{campaign_difficulty_label, campaign_side_lobby_house},
    load_kind::LoadKind,
    original_screen::OriginalScreen,
};

use crate::host::{
    battle_controller::BattleController,
    boot::{self, BootResult},
    load_job::LoadJob,
};

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
        self.ensure_lobby_sides();
        self.load_brief_csf = self
            .lobby_countries
            .iter()
            .find(|c| c.id.eq_ignore_ascii_case(self.skirmish.side.as_str()))
            .map(|c| c.load_brief.clone())
            .filter(|s| !s.is_empty());
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
            req.boot_kind = LoadKind::Skirmish;
            req
        }));
        // load_job 赋值后刷新：禁用重试并改标题提示。
        self.refresh_menu_backdrop();
        self.refresh_shell_title();
    }

    /// 开始战役装载：按选边解析 `battle.ini` 首关并走共用 `LoadScreen`。
    pub(super) fn begin_campaign_load(&mut self, side: &'static str) {
        let Some(camp) = boot::resolve_install_campaign_for_side(side)
        else {
            self.campaign_side = Some(side);
            self.banner = format!("战役表缺少 {side} 首关 · 检查 battle.ini");
            tracing::warn!(side, "战役首关不可解析");
            self.refresh_menu_backdrop();
            self.refresh_shell_title();
            return;
        };
        self.campaign_side = Some(side);
        self.load_brief_csf = if camp.description_csf.is_empty() {
            None
        } else {
            Some(camp.description_csf.to_string())
        };
        self.begin_campaign_scenario_load(camp.scenario.as_str(), Some(camp.id.as_str()));
    }

    /// 按指定 scenario 装载战役局（重开当前关 / 进入 NextMission）。
    ///
    /// 需要已设置 `campaign_side`（决定本地 house 与难度）。
    pub(super) fn begin_campaign_scenario_load(&mut self, scenario: &str, battle_id: Option<&str>) {
        if self.load_job.is_some() || self.pending_load_boot.is_some() {
            tracing::warn!("装载已在进行，忽略重复开始");
            return;
        }
        let Some(side) = self.campaign_side
        else {
            self.banner = "无战役选边 · 无法装载 scenario".into();
            self.set_screen(OriginalScreen::Campaign);
            self.refresh_shell_title();
            return;
        };
        let Some(house) = campaign_side_lobby_house(side)
        else {
            self.banner = format!("未知战役选边 · {side}");
            self.refresh_shell_title();
            return;
        };
        let scenario = scenario.trim();
        if scenario.is_empty() {
            self.banner = "战役 scenario 为空".into();
            self.refresh_shell_title();
            return;
        }

        self.load_kind = LoadKind::Campaign;
        self.skirmish.side = house.to_string();
        self.skirmish.difficulty = campaign_difficulty_label(self.campaign_difficulty).to_string();
        self.selected_map = Some(scenario.to_string());
        self.banner = match battle_id {
            Some(id) => format!("正在装载战役 {id} · {scenario} · {house}…"),
            None => format!("正在装载战役 · {scenario} · {house}…"),
        };
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
            self.ensure_lobby_sides();
            let house_index = self.skirmish.sides.iter().position(|s| s.eq_ignore_ascii_case(house)).unwrap_or(0) as u8;
            let mut req = self.skirmish.clone();
            req.preferred_map = Some(scenario.to_string());
            req.side = house.to_string();
            req.difficulty = campaign_difficulty_label(self.campaign_difficulty).to_string();
            req.row_sides = [house_index; ra_layout::SKIRMISH_ROW_COUNT];
            if req.sides.is_empty() {
                req.set_lobby_sides(vec![house.to_string()]);
                req.row_sides = [0; ra_layout::SKIRMISH_ROW_COUNT];
            }
            req.boot_kind = LoadKind::Campaign;
            req
        }));
        tracing::info!(side, scenario, house, battle_id, "开始战役 scenario 装载");
        self.refresh_menu_backdrop();
        self.refresh_shell_title();
    }

    /// 按当前 `LoadKind` 重试。
    pub(super) fn retry_load(&mut self) {
        if self.load_job.is_some() {
            tracing::info!("装载进行中，忽略重试");
            return;
        }
        match self.load_kind {
            LoadKind::Skirmish => self.begin_skirmish_load(),
            LoadKind::Campaign => {
                if let Some(map) = self.selected_map.clone() {
                    self.begin_campaign_scenario_load(&map, None);
                } else if let Some(side) = self.campaign_side {
                    self.begin_campaign_load(side);
                } else {
                    self.banner = "无战役选边可重试 · Esc 回选边".into();
                    self.refresh_menu_backdrop();
                    self.refresh_shell_title();
                }
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
        self.load_brief_csf = None;
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
            None => {
                self.battle_controller = Some(BattleController::from_boot(boot, self.status_path.clone(), self.test_scene.clone()));
                if let Some(ctrl) = self.battle_controller.as_mut() {
                    ctrl.ensure_start_view(&mut self.renderer);
                }
            }
        }
        self.ensure_lobby_sides();
        let house = self.skirmish.side.clone();
        let faction_id = self
            .lobby_countries
            .iter()
            .find(|c| c.id.eq_ignore_ascii_case(house.as_str()))
            .map(|c| c.side.as_str().to_string())
            .filter(|s| !s.is_empty());
        let chrome = self.resolve_ui_faction_chrome(&house, faction_id.as_deref());
        if let Some(ctrl) = self.battle_controller.as_mut() {
            ctrl.set_ui_faction_side(faction_id);
            ctrl.set_ui_faction_chrome(chrome);
        }
        let ok = self.battle_controller.as_ref().is_some_and(|c| c.has_session());
        let target = self.pending_after_load.take().unwrap_or(OriginalScreen::Battle);
        if ok {
            self.battle_theater_mounted = None;
            self.ensure_battle_theater_mixes();
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
