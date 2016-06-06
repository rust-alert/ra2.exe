//! 遭遇战 / 战役结算页：积分表数据与交互。

use ra_assets::{Palette, ShpFile};
use ra_engine::{BattleOutcome, SessionBootKind};
use ra_widgets::compose::{format_score_time, skirmish_score_hit_at, SkirmishScoreRow};
use ra_widgets::load_kind::LoadKind;
use ra_widgets::skin::decode::frame_to_canvas_rgba;
use ra_widgets::skin::text::resolve_csf_text;
use ra_widgets::skirmish_setup::LOBBY_COLORS;
use ra_renderer::RgbaImage;
use winit::event::{ElementState, MouseButton, WindowEvent};
use winit::keyboard::{KeyCode, PhysicalKey};

use crate::host::battle_controller::BattleNav;

use super::Shell;

impl Shell {
    /// 本机阵营 house 名（驱动积分页左区装载艺术）。
    pub(super) fn results_local_house(&self) -> String {
        self.battle_controller
            .as_ref()
            .and_then(|c| c.session.as_ref())
            .and_then(|s| s.battle())
            .and_then(|g| {
                g.world
                    .players
                    .iter()
                    .find(|p| p.id == g.world.local_player)
                    .map(|p| p.house.to_string())
            })
            .filter(|s| !s.is_empty())
            .unwrap_or_else(|| self.skirmish.side.clone())
    }

    /// 本机阵营对应 rules `Side=`（模组扩展国名时驱动 UI 族）。
    pub(super) fn results_faction_id(&self) -> Option<&str> {
        let house = self.results_local_house();
        self.lobby_countries
            .iter()
            .find(|c| c.id.eq_ignore_ascii_case(house.as_str()))
            .map(|c| c.side.as_str())
            .filter(|s| !s.is_empty())
    }

    /// 惰性解码积分页左区战报图（按 [`ra_widgets::skirmish_setup::UiFactionChrome`]）。
    pub(super) fn ensure_score_backdrop(&mut self) {
        let house = self.results_local_house();
        let faction_owned = self.results_faction_id().map(str::to_string);
        let faction_id = faction_owned.as_deref();
        let Some(chrome) = self
            .battle_controller
            .as_ref()
            .and_then(|c| c.ui_faction_chrome().cloned())
            .or_else(|| self.resolve_ui_faction_chrome(&house, faction_id))
        else {
            return;
        };
        let candidates = chrome.score_background_candidates();
        let pal_names = chrome.score_palette_candidates();
        // 战报图名只来自 rules / edition adaptor，禁止静默回退原版 `mpascrnl`。
        let (Some(want_shp), Some(want_pal)) = (candidates.first(), pal_names.first())
        else {
            tracing::debug!(
                house = %house,
                "积分页战报图未配置（需 MultiplayerScore 或 adaptor stock）"
            );
            self.score_backdrop = None;
            self.score_backdrop_for = None;
            return;
        };
        let want_key = format!(
            "{house}:{}:{}:{want_shp}:{want_pal}",
            faction_id.unwrap_or("-"),
            chrome.mix_file_index
        );
        if self.score_backdrop.is_some() && self.score_backdrop_for.as_deref() == Some(want_key.as_str()) {
            return;
        }
        self.score_backdrop = None;
        self.score_backdrop_for = Some(want_key);
        self.ensure_menu_assets();
        let Some(source) = self.menu_assets.as_ref().and_then(|a| a.source.as_ref())
        else {
            return;
        };
        for name in &candidates {
            let Some(hit) = source.resolve(name)
            else {
                continue;
            };
            let Ok(shp) = ShpFile::parse(&hit.bytes)
            else {
                continue;
            };
            let Some(frame) = shp.frames.first()
            else {
                continue;
            };
            let mut decoded = None;
            for paln in &pal_names {
                let Some(ph) = source.resolve(paln)
                else {
                    continue;
                };
                let Ok(pal) = Palette::parse(&ph.bytes)
                else {
                    continue;
                };
                if let Some(img) = frame_to_canvas_rgba(&shp, frame, &pal) {
                    tracing::info!(
                        %name,
                        %paln,
                        house = %house,
                        w = img.width(),
                        h = img.height(),
                        "已装载积分页战报图"
                    );
                    decoded = Some(img);
                    break;
                }
            }
            if let Some(img) = decoded {
                self.score_backdrop = Some(img);
                return;
            }
        }
        tracing::debug!(house = %house, "积分页战报图不可读");
    }

    /// 从当前对局快照拼积分表行。
    pub(super) fn skirmish_score_rows(&self) -> Vec<SkirmishScoreRow> {
        let Some(game) = self
            .battle_controller
            .as_ref()
            .and_then(|c| c.session.as_ref())
            .and_then(|s| s.battle())
        else {
            return Vec::new();
        };
        let local_house = game
            .world
            .players
            .iter()
            .find(|p| p.id == game.world.local_player)
            .map(|p| p.house.to_string())
            .unwrap_or_default();
        let ai_label = resolve_csf_text(self.menu_csf.as_ref(), "GUI:AI")
            .unwrap_or_else(|| "电脑".into());
        let stats_players = game
            .battle_stats
            .as_ref()
            .map(|s| s.players.as_slice())
            .unwrap_or(&[]);
        if !stats_players.is_empty() {
            return stats_players
                .iter()
                .enumerate()
                .map(|(i, row)| {
                    let is_local = row.house.eq_ignore_ascii_case(&local_house);
                    let name = if is_local {
                        self.skirmish.player_name.clone()
                    } else {
                        ai_label.clone()
                    };
                    let rgb = LOBBY_COLORS
                        .get(i % LOBBY_COLORS.len())
                        .copied()
                        .unwrap_or([220, 220, 220]);
                    let color = [rgb[0], rgb[1], rgb[2], 255];
                    SkirmishScoreRow {
                        name,
                        color,
                        kills: row.kills,
                        losses: row.losses,
                        built: row.built,
                        score: row.score,
                    }
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
            .map(|(i, p)| {
                let is_local = p.house.eq_ignore_ascii_case(&local_house);
                let name = if is_local {
                    self.skirmish.player_name.clone()
                } else {
                    ai_label.clone()
                };
                let rgb = LOBBY_COLORS
                    .get(i % LOBBY_COLORS.len())
                    .copied()
                    .unwrap_or([220, 220, 220]);
                let losses = losses_by_house.get(p.house.as_ref()).copied().unwrap_or(0);
                let kills = p.kills;
                let built = p.built;
                let score = (p.funds_spent / 100) + (kills as i32) * 10 - (losses as i32) * 5;
                SkirmishScoreRow {
                    name,
                    color: [rgb[0], rgb[1], rgb[2], 255],
                    kills,
                    losses,
                    built,
                    score,
                }
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
            WindowEvent::MouseInput {
                state,
                button: MouseButton::Left,
                ..
            } => {
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
                        if pressed == Some("continue") && hit == Some("continue") {
                            self.results_confirm_nav()
                        } else {
                            BattleNav::None
                        }
                    }
                }
            }
            WindowEvent::KeyboardInput { event: key_ev, .. } if key_ev.state == ElementState::Pressed => {
                match key_ev.physical_key {
                    PhysicalKey::Code(KeyCode::Enter) | PhysicalKey::Code(KeyCode::NumpadEnter) => {
                        self.results_confirm_nav()
                    }
                    PhysicalKey::Code(KeyCode::Escape) => BattleNav::ToMainMenu,
                    _ => BattleNav::None,
                }
            }
            _ => BattleNav::None,
        }
    }

    /// 将窗口光标映射到壳层设计坐标后命中「继续」。
    fn results_continue_hit(&self) -> Option<&'static str> {
        let (win_w, win_h) = self.display_mode.size();
        let (sx, sy) = ra_layout::window_to_shell_px(
            self.cursor.0,
            self.cursor.1,
            win_w as f64,
            win_h as f64,
        );
        skirmish_score_hit_at(sx, sy)
    }

    /// Enter / 继续：战役有对应续关字段则下一关，否则离开结算。
    ///
    /// 胜 → `[Basic] NextMission`；败 → `[Basic] AlternateNextMission`。
    fn results_confirm_nav(&self) -> BattleNav {
        let continue_campaign = self
            .battle_controller
            .as_ref()
            .and_then(|c| c.session.as_ref())
            .and_then(|s| s.battle())
            .is_some_and(|g| {
                if g.boot_kind != SessionBootKind::Campaign {
                    return false;
                }
                match g.outcome.as_ref() {
                    Some(BattleOutcome::Victory { .. }) => g.world.map.campaign_continue_scenario(true).is_some(),
                    Some(BattleOutcome::Defeat { .. }) => g.world.map.campaign_continue_scenario(false).is_some(),
                    None => false,
                }
            });
        if continue_campaign {
            BattleNav::ContinueCampaign
        } else {
            BattleNav::ToMainMenu
        }
    }
}
