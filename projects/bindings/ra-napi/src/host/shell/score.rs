//! 遭遇战 / 战役结算页：积分表数据与交互。

use ra_engine::{BattleOutcome, SessionBootKind, is_ambient_house};
use ra_widgets::{
    compose::{SkirmishScoreRow, campaign_score_continue_hit_at, format_score_time, skirmish_score_hit_at},
    load_kind::LoadKind,
    skin::text::resolve_csf_text,
    skirmish_setup::LOBBY_COLORS,
};
use winit::{
    event::{ElementState, MouseButton, WindowEvent},
    keyboard::{KeyCode, PhysicalKey},
};

use crate::host::battle_controller::BattleNav;

use super::Shell;

impl Shell {
    /// 从当前对局快照拼积分表行。
    ///
    /// - 战役：只显示本地玩家一行（原版任务积分，不把脚本房主刷成「电脑」表）。
    /// - 遭遇战：排除 Neutral/Civilian 氛围房，其余席位入表。
    pub(super) fn skirmish_score_rows(&self) -> Vec<SkirmishScoreRow> {
        let Some(game) = self.battle_controller.as_ref().and_then(|c| c.session.as_ref()).and_then(|s| s.battle())
        else {
            return Vec::new();
        };
        let campaign = self.results_is_campaign();
        let local_house = game.world.players.iter().find(|p| p.id == game.world.local_player).map(|p| p.house.to_string()).unwrap_or_default();
        let ai_label = resolve_csf_text(self.menu_csf.as_ref(), "GUI:AI").unwrap_or_else(|| "电脑".into());
        let include_house = |house: &str| -> bool {
            if campaign {
                return house.eq_ignore_ascii_case(&local_house);
            }
            !is_ambient_house(house)
        };
        let stats_players = game.battle_stats.as_ref().map(|s| s.players.as_slice()).unwrap_or(&[]);
        if !stats_players.is_empty() {
            return stats_players
                .iter()
                .enumerate()
                .filter(|(_, row)| include_house(row.house.as_ref()))
                .map(|(i, row)| {
                    let is_local = row.house.eq_ignore_ascii_case(&local_house);
                    let name = if is_local { self.skirmish.player_name.clone() } else { ai_label.clone() };
                    let rgb = LOBBY_COLORS.get(i % LOBBY_COLORS.len()).copied().unwrap_or([220, 220, 220]);
                    let color = [rgb[0], rgb[1], rgb[2], 255];
                    SkirmishScoreRow { name, color, kills: row.kills, losses: row.losses, built: row.built, score: row.score }
                })
                .collect();
        }
        // 无逐玩家统计时回退：按权威 `PlayerState` 填 kills/built，并扫死亡实体估 losses。
        let mut losses_by_house: std::collections::BTreeMap<String, u32> = std::collections::BTreeMap::new();
        for id in game.world.entity_ids() {
            let Some((_, _, dead)) = game.world.ecs_health(id)
            else {
                continue;
            };
            if !dead {
                continue;
            }
            let Some(owner) = game.world.ecs_owner(id)
            else {
                continue;
            };
            let Some((_, kind)) = game.world.ecs_identity(id)
            else {
                continue;
            };
            if matches!(
                kind,
                ra_map::MapEntityKind::Structure
                    | ra_map::MapEntityKind::Unit
                    | ra_map::MapEntityKind::Infantry
                    | ra_map::MapEntityKind::Aircraft
            ) {
                *losses_by_house.entry(owner.to_string()).or_default() += 1;
            }
        }
        game.world
            .players
            .iter()
            .enumerate()
            .filter(|(_, p)| include_house(p.house.as_ref()))
            .map(|(i, p)| {
                let is_local = p.house.eq_ignore_ascii_case(&local_house);
                let name = if is_local { self.skirmish.player_name.clone() } else { ai_label.clone() };
                let rgb = LOBBY_COLORS.get(i % LOBBY_COLORS.len()).copied().unwrap_or([220, 220, 220]);
                let losses = losses_by_house.get(p.house.as_ref()).copied().unwrap_or(0);
                let kills = p.kills;
                let built = p.built;
                let score = (p.funds_spent / 100) + (kills as i32) * 10 - (losses as i32) * 5;
                SkirmishScoreRow { name, color: [rgb[0], rgb[1], rgb[2], 255], kills, losses, built, score }
            })
            .collect()
    }

    /// 结算时长文案。
    pub(super) fn skirmish_score_time_text(&self) -> String {
        let ticks = self
            .battle_controller
            .as_ref()
            .and_then(|c| c.session.as_ref())
            .and_then(|s| s.battle())
            .and_then(|g| g.battle_stats.as_ref())
            .map(|s| s.duration_ticks)
            .unwrap_or(0);
        format_score_time(ticks, 15)
    }

    /// 是否战役结算（标题走任务积分）。
    pub(super) fn results_is_campaign(&self) -> bool {
        self.load_kind == LoadKind::Campaign
            || self
                .battle_controller
                .as_ref()
                .and_then(|c| c.session.as_ref())
                .and_then(|s| s.battle())
                .is_some_and(|g| g.boot_kind == SessionBootKind::Campaign)
    }

    /// 离开结算页时清掉战役战报图缓存。
    pub(super) fn clear_campaign_score_art(&mut self) {
        self.campaign_score_background = None;
        self.campaign_score_transition_frames.clear();
        self.campaign_score_transition_frame = 0;
        self.campaign_score_transition_clock = None;
        self.campaign_score_transition_accum = 0.0;
        self.campaign_score_art_tried = false;
    }

    /// 进战役结算时从第 0 帧重播过渡。
    pub(super) fn reset_campaign_score_transition_anim(&mut self) {
        self.campaign_score_transition_frame = 0;
        self.campaign_score_transition_clock = None;
        self.campaign_score_transition_accum = 0.0;
    }

    /// 当前应绘制的过渡帧；播完后停在末帧。
    pub(super) fn campaign_score_transition_sprite(&self) -> Option<&ra_widgets::skin::decode::DecodedUiSprite> {
        if self.campaign_score_transition_frames.is_empty() {
            return None;
        }
        let last = self.campaign_score_transition_frames.len() - 1;
        self.campaign_score_transition_frames.get(self.campaign_score_transition_frame.min(last))
    }

    /// 战役结算过渡 10 FPS；未播完才进帧。返回是否需要重绘。
    pub(super) fn tick_campaign_score_transition_anim(&mut self) -> bool {
        let n = self.campaign_score_transition_frames.len();
        if n <= 1 || self.campaign_score_transition_frame >= n - 1 {
            self.campaign_score_transition_clock = None;
            return false;
        }
        const FRAME_SECS: f64 = 0.1;
        let dt = self
            .campaign_score_transition_clock
            .replace(std::time::Instant::now())
            .map(|t0| t0.elapsed().as_secs_f64())
            .unwrap_or(0.0)
            .min(0.25);
        self.campaign_score_transition_accum += dt;
        let mut advanced = false;
        while self.campaign_score_transition_accum >= FRAME_SECS && self.campaign_score_transition_frame < n - 1 {
            self.campaign_score_transition_accum -= FRAME_SECS;
            self.campaign_score_transition_frame += 1;
            advanced = true;
        }
        if self.campaign_score_transition_frame >= n - 1 {
            self.campaign_score_transition_clock = None;
            self.campaign_score_transition_accum = 0.0;
        }
        advanced
    }

    /// 合成战役结算页：侧栏 hub + `CampaignScore.Background` + `Transition` 动画 + 「继续」。
    ///
    /// **不是**壳层 `mnscrnl` / `compose_skirmish_score_page`。
    pub(super) fn compose_campaign_results_page(&mut self) -> Option<ra_renderer::RgbaImage> {
        self.ensure_campaign_score_art();
        self.ensure_menu_assets();
        let assets = self.menu_assets.as_ref().and_then(|a| a.source.as_ref());
        if let Some(ctrl) = self.battle_controller.as_mut() {
            ctrl.ensure_battle_hud_chrome(assets);
            ctrl.ensure_pause_menu_chrome(assets);
        }
        let funds = self
            .battle_controller
            .as_ref()
            .and_then(|c| c.session.as_ref())
            .and_then(|s| s.battle())
            .and_then(|g| g.world.players.iter().find(|p| p.id == g.world.local_player).map(|p| p.funds));
        let pause = self.battle_controller.as_ref().and_then(|c| c.pause_menu_chrome.as_ref());
        let hud = self.battle_controller.as_ref().and_then(|c| c.hud_chrome.as_ref());
        ra_widgets::compose::compose_campaign_score_overlay(
            self.window_width as u32,
            self.window_height as u32,
            self.menu_pressed_entry,
            self.menu_hovered_entry,
            self.menu_font.as_ref(),
            self.menu_csf.as_ref(),
            self.campaign_score_background.as_ref(),
            self.campaign_score_transition_sprite(),
            pause,
            hud,
            funds,
        )
    }

    /// 解码战役结算 `CampaignScore.Background` + `Transition` 全帧（仅战役 Results 需要）。
    pub(super) fn ensure_campaign_score_art(&mut self) {
        if self.campaign_score_art_tried || !self.results_is_campaign() {
            return;
        }
        self.campaign_score_art_tried = true;
        self.ensure_menu_assets();
        let Some(source) = self.menu_assets.as_ref().and_then(|a| a.source.as_ref())
        else {
            tracing::warn!("战役结算缺挂载源，无法解码 CampaignScore");
            return;
        };
        let side = self.results_score_side_id();
        let Some(chrome) = self.resolve_ui_faction_chrome(side.as_str(), Some(side.as_str()))
        else {
            tracing::warn!(%side, "战役结算缺 UiFactionChrome");
            return;
        };
        let pals = ra_widgets::skirmish_setup::campaign_score_screen_palette_candidates(&chrome);
        let Some(pal) = pals.into_iter().find(|p| source.resolve(p).is_some())
        else {
            tracing::warn!(%side, "战役结算缺可读 CampaignScore.Palette");
            return;
        };
        let bgs = ra_widgets::skirmish_setup::campaign_score_screen_background_candidates(&chrome);
        for bg in bgs {
            if source.resolve(&bg).is_none() {
                continue;
            }
            let asset = ra_widgets::screens::page::UiAssetRef::with_palette(&bg, &pal);
            match ra_widgets::skin::decode::decode_asset_ref(source, &asset) {
                Ok(sprite) => {
                    tracing::info!(%bg, %pal, "战役结算 Background 已解码");
                    self.campaign_score_background = Some(sprite);
                    break;
                }
                Err(e) => tracing::warn!(%bg, %pal, "CampaignScore.Background 解码失败 · {e}"),
            }
        }
        let transitions = ra_widgets::skirmish_setup::campaign_score_screen_transition_candidates(&chrome);
        for name in transitions {
            if source.resolve(&name).is_none() {
                continue;
            }
            let asset = ra_widgets::screens::page::UiAssetRef::with_palette(&name, &pal);
            match ra_widgets::skin::decode::decode_asset_frames(source, &asset) {
                Ok(frames) if !frames.is_empty() => {
                    tracing::info!(%name, %pal, frames = frames.len(), "战役结算 Transition 全帧已解码");
                    self.campaign_score_transition_frames = frames;
                    self.reset_campaign_score_transition_anim();
                    break;
                }
                Ok(_) => tracing::warn!(%name, "CampaignScore.Transition 无帧"),
                Err(e) => tracing::warn!(%name, %pal, "CampaignScore.Transition 解码失败 · {e}"),
            }
        }
        if self.campaign_score_background.is_none() {
            tracing::warn!(%side, "战役结算 Background 未就绪");
        }
        if self.campaign_score_transition_frames.is_empty() {
            tracing::warn!(%side, "战役结算 Transition 未就绪");
        }
    }

    /// 结算页选用哪一 Side 的战报图（`[Sides]` id：`GDI`/`Nod`/…）。
    pub(super) fn results_score_side_id(&self) -> String {
        if self.results_is_campaign() {
            if let Some(raw) = self.campaign_side {
                let key = raw.to_ascii_lowercase();
                if matches!(key.as_str(), "soviet" | "russia" | "russians" | "nod") {
                    return "Nod".into();
                }
                if matches!(key.as_str(), "allied" | "americans" | "tutorial" | "gdi") {
                    return "GDI".into();
                }
                if self.lobby_side_chromes.iter().any(|c| c.id.eq_ignore_ascii_case(raw)) {
                    return raw.to_string();
                }
            }
        }
        let country = self.skirmish.side.as_str();
        if let Some(c) = self.lobby_countries.iter().find(|c| c.id.eq_ignore_ascii_case(country)) {
            let side = c.side.as_str();
            if !side.is_empty() {
                return side.to_string();
            }
        }
        if self.lobby_side_chromes.iter().any(|c| c.id.eq_ignore_ascii_case(country)) {
            return country.to_string();
        }
        "GDI".into()
    }

    /// 结算页输入：继续 / 离开；战役胜且有下一关时 Enter=下一关。
    pub(super) fn handle_results_event(&mut self, event: &WindowEvent) -> BattleNav {
        match event {
            WindowEvent::CursorMoved { position, .. } => {
                let scale = self.window.as_ref().map(|w| w.scale_factor()).unwrap_or(1.0);
                let logical = position.to_logical::<f64>(scale);
                self.cursor = (logical.x, logical.y);
                let hit = self.results_continue_hit();
                if self.menu_hovered_entry != hit {
                    self.menu_hovered_entry = hit;
                    self.refresh_menu_backdrop();
                }
                BattleNav::None
            }
            WindowEvent::MouseInput { state, button: MouseButton::Left, .. } => {
                let hit = self.results_continue_hit();
                match state {
                    ElementState::Pressed => {
                        if hit.is_some() {
                            self.play_menu_click();
                        }
                        self.menu_pressed_entry = hit;
                        self.refresh_menu_backdrop();
                        BattleNav::None
                    }
                    ElementState::Released => {
                        let pressed = self.menu_pressed_entry.take();
                        self.refresh_menu_backdrop();
                        if pressed == Some("continue") && hit == Some("continue") { self.results_confirm_nav() } else { BattleNav::None }
                    }
                }
            }
            WindowEvent::KeyboardInput { event: key_ev, .. } if key_ev.state == ElementState::Pressed => match key_ev.physical_key {
                PhysicalKey::Code(KeyCode::Enter) | PhysicalKey::Code(KeyCode::NumpadEnter) => self.results_confirm_nav(),
                PhysicalKey::Code(KeyCode::Escape) => BattleNav::ToMainMenu,
                _ => BattleNav::None,
            },
            _ => BattleNav::None,
        }
    }

    /// 命中「继续」：战役走暂停 hub 底钮几何；遭遇战走壳层积分钮。
    fn results_continue_hit(&self) -> Option<&'static str> {
        if self.results_is_campaign() {
            let hud = self.battle_controller.as_ref().and_then(|c| c.hud_chrome.as_ref());
            return campaign_score_continue_hit_at(self.window_width as u32, self.window_height as u32, hud, self.cursor.0, self.cursor.1);
        }
        let (win_w, win_h) = self.display_mode.size();
        let (sx, sy) = ra_layout::window_to_shell_px(self.cursor.0, self.cursor.1, win_w as f64, win_h as f64);
        skirmish_score_hit_at(sx, sy)
    }

    /// Enter / 继续：战役有胜负则走续关导航（由 `apply_nav` 解析 `battle.ini`）；遭遇战回大厅。
    fn results_confirm_nav(&self) -> BattleNav {
        if !self.results_is_campaign() {
            return BattleNav::ToMainMenu;
        }
        let has_outcome = self
            .battle_controller
            .as_ref()
            .and_then(|c| c.session.as_ref())
            .and_then(|s| s.battle())
            .is_some_and(|g| g.boot_kind == SessionBootKind::Campaign && g.outcome.is_some());
        if has_outcome {
            BattleNav::ContinueCampaign
        }
        else {
            tracing::warn!("战役结算无胜负结果，无法续关");
            BattleNav::ToMainMenu
        }
    }

    /// 解析战役「继续」目标 scenario（胜：`battle.ini` 同线下一关；败：地图 Alt 字段）。
    pub(super) fn resolve_continue_campaign_scenario(&self) -> Option<String> {
        let game = self.battle_controller.as_ref().and_then(|c| c.session.as_ref()).and_then(|s| s.battle())?;
        if game.boot_kind != SessionBootKind::Campaign {
            return None;
        }
        let victory = match game.outcome.as_ref() {
            Some(BattleOutcome::Victory { .. }) => true,
            Some(BattleOutcome::Defeat { .. }) => false,
            None => return None,
        };
        let current =
            self.campaign_scenario.as_deref().map(str::trim).filter(|s| !s.is_empty()).unwrap_or_else(|| game.world.map.name.as_str());
        crate::host::boot::resolve_campaign_continue_scenario(
            current,
            victory,
            game.world.map.campaign_continue_scenario(true),
            game.world.map.campaign_continue_scenario(false),
        )
    }
}
