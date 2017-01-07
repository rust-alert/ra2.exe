//! 遭遇战 / 战役结算页：积分表数据与交互。

use ra_engine::{BattleOutcome, SessionBootKind, is_ambient_house};
use ra_widgets::{
    compose::{SkirmishScoreRow, format_score_time, skirmish_score_hit_at},
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

    /// 将窗口光标映射到壳层设计坐标后命中「继续」。
    fn results_continue_hit(&self) -> Option<&'static str> {
        let (win_w, win_h) = self.display_mode.size();
        let (sx, sy) = ra_layout::window_to_shell_px(self.cursor.0, self.cursor.1, win_w as f64, win_h as f64);
        skirmish_score_hit_at(sx, sy)
    }

    /// Enter / 继续：战役有对应续关字段则下一关，否则离开结算。
    ///
    /// 胜 → `[Basic] NextMission`；败 → `[Basic] AlternateNextMission`。
    fn results_confirm_nav(&self) -> BattleNav {
        let continue_campaign = self.battle_controller.as_ref().and_then(|c| c.session.as_ref()).and_then(|s| s.battle()).is_some_and(|g| {
            if g.boot_kind != SessionBootKind::Campaign {
                return false;
            }
            match g.outcome.as_ref() {
                Some(BattleOutcome::Victory { .. }) => g.world.map.campaign_continue_scenario(true).is_some(),
                Some(BattleOutcome::Defeat { .. }) => g.world.map.campaign_continue_scenario(false).is_some(),
                None => false,
            }
        });
        if continue_campaign { BattleNav::ContinueCampaign } else { BattleNav::ToMainMenu }
    }
}
