//! 对局页控制器：输入意图、命令、tick、快照；不含窗口与页面导航外壳。

use std::sync::Arc;

use ra_engine::{BattleCapabilitiesSnapshot, CapabilityItem};
use ra_layout::{BattleHudChromeMetrics, SIDEBAR_TAB_COUNT, cameo_visible_slot_count, rect_px_from_snapshot, solve_battle_hud_with_metrics};
use ra_renderer::Renderer;
use ra_widgets::{
    battle_hud::{BattleHudHit, hit_at_with_chrome},
    skin::text::command_button_csf_tooltip,
};
use winit::window::Window;

use super::{BattleController, BattleNav};

impl BattleController {
    /// 战术区悬停的本方实体（移动优先，其次建筑）。
    pub(super) fn tactical_hover_entity(&self, renderer: &Renderer, window: Option<&Arc<Window>>) -> Option<ra_types::EntityId> {
        let window = window?;
        let game = self.session.as_ref()?.battle()?;
        let vp = self.map_viewport(window);
        let (wx, wy) = vp.screen_to_world(renderer.camera(), self.cursor.0 as f32, self.cursor.1 as f32);
        game.pick_local_mobile_near_image(wx, wy, 72.0).or_else(|| Self::pick_local_building_at_image(game, wx, wy))
    }

    pub(super) fn hud_snap_for_window(&self, window: &Window) -> ra_layout::LayoutSnapshot {
        let size = window.inner_size();
        let w = size.width.max(1);
        let h = size.height.max(1);
        let metrics = self.hud_chrome.as_ref().map(|c| BattleHudChromeMetrics::for_mix(&c.mix)).unwrap_or_else(BattleHudChromeMetrics::sidec01);
        solve_battle_hud_with_metrics(w, h, metrics)
    }

    pub(super) fn hit_hud_at(&self, window: &Window, x: i32, y: i32) -> Option<BattleHudHit> {
        let snap = self.hud_snap_for_window(window);
        let metrics = self.hud_chrome.as_ref().map(|c| BattleHudChromeMetrics::for_mix(&c.mix)).unwrap_or_else(BattleHudChromeMetrics::sidec01);
        let band = rect_px_from_snapshot(&snap, "cameo_band");
        let visible = cameo_visible_slot_count(band.h);
        let cameo_count = self.current_tab_cameo_count(visible);
        let hit = hit_at_with_chrome(&snap, self.hud_chrome.as_ref(), metrics.power_w, cameo_count, x, y);
        if let Some(BattleHudHit::SidebarTab(tab)) = hit {
            let tabs_visible = Self::sidebar_tabs_visible(self.current_capabilities().as_ref());
            if !tabs_visible.get(tab).copied().unwrap_or(false) {
                return None;
            }
        }
        hit
    }

    pub(super) fn tab_items(caps: &BattleCapabilitiesSnapshot, tab: usize) -> Vec<CapabilityItem> {
        match tab.min(SIDEBAR_TAB_COUNT.saturating_sub(1)) {
            0 => caps.build_items.clone(),
            1 => caps.defense_items.clone(),
            2 => caps.infantry_items.clone(),
            3 => {
                let mut items = caps.vehicle_items.clone();
                items.extend(caps.aircraft_items.iter().cloned());
                items
            }
            _ => Vec::new(),
        }
    }

    /// 无对应可建造基础的分类页签不显示。
    ///
    /// Q/W 建筑与防御均依赖建造场；E 步兵依赖兵营；R 载具或飞行器厂。
    pub(super) fn sidebar_tabs_visible(caps: Option<&BattleCapabilitiesSnapshot>) -> [bool; SIDEBAR_TAB_COUNT] {
        let Some(caps) = caps
        else {
            return [false; SIDEBAR_TAB_COUNT];
        };
        [
            caps.has_construction_yard,
            caps.has_construction_yard,
            caps.has_infantry_factory,
            caps.has_vehicle_factory || caps.has_aircraft_factory,
        ]
    }

    /// 热键切页签（`keyboard.ini` Structure/Defense/Infantry/UnitTab）。落位须点闪烁 cameo，不在切页时自动进入。
    pub(super) fn hotkey_sidebar_tab(&mut self, tab: usize) {
        let tab = tab.min(SIDEBAR_TAB_COUNT.saturating_sub(1));
        let visible = Self::sidebar_tabs_visible(self.current_capabilities().as_ref());
        if !visible.get(tab).copied().unwrap_or(false) {
            return;
        }
        if self.sidebar_tab != tab {
            self.sidebar_tab = tab;
            self.cameo_scroll = 0;
            if tab > 1 {
                self.place_mode = None;
            }
            tracing::info!("侧栏页签 · {tab}（热键）");
        }
    }

    /// 当前页签若已无基础，切到第一个仍可见的页签。
    pub(super) fn sync_sidebar_tab_to_visible(&mut self, visible: [bool; SIDEBAR_TAB_COUNT]) {
        if visible.get(self.sidebar_tab).copied().unwrap_or(false) {
            return;
        }
        let next = visible.iter().position(|&v| v).unwrap_or(0);
        if self.sidebar_tab != next {
            self.sidebar_tab = next;
            self.cameo_scroll = 0;
            if next > 1 {
                self.place_mode = None;
            }
        }
    }

    pub(super) fn current_capabilities(&self) -> Option<BattleCapabilitiesSnapshot> {
        let game = self.session.as_ref().and_then(|s| s.battle())?;
        Some(game.snapshot_capabilities(&self.local.selected))
    }

    pub(super) fn current_tab_cameo_count(&self, visible_slots: usize) -> usize {
        let Some(caps) = self.current_capabilities()
        else {
            return 0;
        };
        let total = Self::tab_items(&caps, self.sidebar_tab).len();
        total.min(visible_slots)
    }

    pub(super) fn clamp_cameo_scroll(&mut self, visible_slots: usize) {
        let Some(caps) = self.current_capabilities()
        else {
            self.cameo_scroll = 0;
            return;
        };
        let total = Self::tab_items(&caps, self.sidebar_tab).len();
        let max_scroll = total.saturating_sub(visible_slots);
        if self.cameo_scroll > max_scroll {
            self.cameo_scroll = max_scroll;
        }
    }

    pub(super) fn cursor_over_cameo_band(&self, window: &Window) -> bool {
        let snap = self.hud_snap_for_window(window);
        let band = rect_px_from_snapshot(&snap, "cameo_band");
        band.w > 0 && band.h > 0 && band.contains(self.cursor.0 as i32, self.cursor.1 as i32)
    }

    pub(super) fn scroll_cameos(&mut self, window: &Window, steps: i32) {
        let snap = self.hud_snap_for_window(window);
        let band = rect_px_from_snapshot(&snap, "cameo_band");
        let visible = cameo_visible_slot_count(band.h);
        let next = self.cameo_scroll as i32 + steps;
        self.cameo_scroll = next.max(0) as usize;
        self.clamp_cameo_scroll(visible);
    }

    /// `Home`/`End`：cameo 列表滚到顶或底（`keyboard.ini` LeftSidebarUp/Down）。
    pub(super) fn jump_cameo_scroll(&mut self, window: &Window, to_end: bool) {
        let snap = self.hud_snap_for_window(window);
        let band = rect_px_from_snapshot(&snap, "cameo_band");
        let visible = cameo_visible_slot_count(band.h);
        if to_end {
            let total = self.current_capabilities().map(|caps| Self::tab_items(&caps, self.sidebar_tab).len()).unwrap_or(0);
            self.cameo_scroll = total.saturating_sub(visible);
        }
        else {
            self.cameo_scroll = 0;
        }
        self.clamp_cameo_scroll(visible);
    }

    pub(super) fn on_sidebar_hit(&mut self, hit: BattleHudHit) -> BattleNav {
        match hit {
            BattleHudHit::SidebarTab(tab) => {
                let tab = tab.min(SIDEBAR_TAB_COUNT.saturating_sub(1));
                let visible = Self::sidebar_tabs_visible(self.current_capabilities().as_ref());
                if !visible.get(tab).copied().unwrap_or(false) {
                    return BattleNav::None;
                }
                if self.sidebar_tab != tab {
                    self.sidebar_tab = tab;
                    self.cameo_scroll = 0;
                    if tab > 1 {
                        self.place_mode = None;
                    }
                    tracing::info!("侧栏页签 · {tab}");
                }
                BattleNav::None
            }
            BattleHudHit::Cameo(slot) => {
                let Some(caps) = self.current_capabilities()
                else {
                    return BattleNav::None;
                };
                let items = Self::tab_items(&caps, self.sidebar_tab);
                let index = self.cameo_scroll.saturating_add(slot);
                let Some(item) = items.get(index)
                else {
                    return BattleNav::None;
                };
                if !item.enabled {
                    if let Some(reason) = item.disabled_reason {
                        tracing::info!("建造栏不可用 · {} · {}", item.type_id, reason.as_hud_label());
                    }
                    return BattleNav::None;
                }
                match self.sidebar_tab {
                    0 | 1 => {
                        let type_id = item.type_id.as_ref();
                        if let Some(game) = self.session.as_ref().and_then(|s| s.battle()) {
                            if game.is_local_ready_to_place(type_id) {
                                if self.place_mode.as_deref() == Some(type_id) {
                                    self.place_mode = None;
                                    tracing::info!("建造模式 · 已关闭");
                                }
                                else {
                                    self.repair_mode = false;
                                    self.sell_mode = false;
                                    self.planning_mode = false;
                                    self.planning_waypoints.clear();
                                    self.place_mode = Some(type_id.to_string());
                                    tracing::info!("建造模式 · 放置 {type_id}（点地图落地，右键/Esc 取消）");
                                }
                                return BattleNav::None;
                            }
                            if game.is_local_producing(type_id) {
                                if let Some(game) = self.session.as_mut().and_then(|s| s.battle_mut()) {
                                    tracing::info!("取消建造 · {type_id}");
                                    game.order_cancel_produce(type_id.to_string());
                                }
                                self.place_mode = None;
                                return BattleNav::None;
                            }
                        }
                        if let Some(game) = self.session.as_mut().and_then(|s| s.battle_mut()) {
                            tracing::info!("开始建造 · {type_id}");
                            game.order_produce(type_id.to_string());
                        }
                        self.place_mode = None;
                    }
                    2 | 3 => {
                        let type_id = item.type_id.as_ref().to_string();
                        if let Some(game) = self.session.as_mut().and_then(|s| s.battle_mut()) {
                            if game.is_local_producing(&type_id) {
                                tracing::info!("取消生产 · {type_id}");
                                game.order_cancel_produce(type_id);
                            }
                            else {
                                tracing::info!("生产 · {type_id}");
                                game.order_produce(type_id);
                            }
                        }
                    }
                    _ => {}
                }
                BattleNav::None
            }
            BattleHudHit::Options => {
                tracing::info!("侧栏 · 打开选项");
                BattleNav::OpenOptions
            }
            BattleHudHit::Repair => {
                self.sell_mode = false;
                self.planning_mode = false;
                self.planning_waypoints.clear();
                self.repair_mode = !self.repair_mode;
                if self.repair_mode {
                    self.place_mode = None;
                }
                tracing::info!(active = self.repair_mode, "侧栏 · 修理工具");
                BattleNav::None
            }
            BattleHudHit::Sell => {
                self.repair_mode = false;
                self.planning_mode = false;
                self.planning_waypoints.clear();
                self.sell_mode = !self.sell_mode;
                if self.sell_mode {
                    self.place_mode = None;
                }
                tracing::info!(active = self.sell_mode, "侧栏 · 出售工具");
                BattleNav::None
            }
            BattleHudHit::Diplomacy => {
                self.log_diplomacy_allies();
                BattleNav::None
            }
            BattleHudHit::CommandButton(_) => BattleNav::None,
        }
    }

    /// 外交钮：只读列出本机同盟（遭遇战改盟待接）。
    pub(super) fn log_diplomacy_allies(&self) {
        let Some(game) = self.session.as_ref().and_then(|s| s.battle())
        else {
            tracing::info!("外交 · 无对局");
            return;
        };
        let Some(local) = game.world.players.iter().find(|p| p.id == game.world.local_player)
        else {
            tracing::info!("外交 · 无本机玩家");
            return;
        };
        let allies = if local.allies.is_empty() { "无".to_string() } else { local.allies.join(", ") };
        let others: Vec<String> = game
            .world
            .players
            .iter()
            .filter(|p| p.id != local.id)
            .map(|p| {
                let allied = local.allies.iter().any(|a| a.eq_ignore_ascii_case(p.house.as_ref()))
                    || p.allies.iter().any(|a| a.eq_ignore_ascii_case(local.house.as_ref()));
                format!("{}={}", p.house, if allied { "同盟" } else { "敌对" })
            })
            .collect();
        let roster = if others.is_empty() { "仅本机".to_string() } else { others.join(", ") };
        tracing::info!(
            house = %local.house,
            allies = %allies,
            roster = %roster,
            "外交 · 同盟只读（遭遇战改盟待接）"
        );
    }

    /// 关闭建造放置 / 修理 / 出售工具。有任一处于激活则返回 `true`。
    pub(super) fn clear_sidebar_tool_modes(&mut self) -> bool {
        let mut cleared = false;
        if self.place_mode.take().is_some() {
            tracing::info!("建造模式 · 已关闭");
            cleared = true;
        }
        if self.repair_mode {
            self.repair_mode = false;
            tracing::info!(active = false, "侧栏 · 修理工具");
            cleared = true;
        }
        if self.sell_mode {
            self.sell_mode = false;
            tracing::info!(active = false, "侧栏 · 出售工具");
            cleared = true;
        }
        if self.planning_mode {
            self.planning_mode = false;
            self.planning_waypoints.clear();
            tracing::info!(active = false, "命令条 · 路径点规划（已丢弃航点）");
            cleared = true;
        }
        cleared
    }

    pub(super) fn refresh_command_hover(&mut self, window: &Window) {
        let x = self.cursor.0 as i32;
        let y = self.cursor.1 as i32;
        let next = match self.hit_hud_at(window, x, y) {
            Some(BattleHudHit::CommandButton(slot)) => Some(slot),
            _ => None,
        };
        self.command_hover = next;
    }

    pub(super) fn on_command_button(&mut self, slot: usize) {
        let name = ra_widgets::skin::text::SKIRMISH_COMMAND_BAR.get(slot).copied().unwrap_or("?");
        let tip = command_button_csf_tooltip(slot).unwrap_or("?");
        tracing::info!(slot, name, tip, "命令条按钮");
        match name {
            "Deploy" => self.deploy_selection(),
            "Guard" => self.guard_selection(),
            "TypeSelect" => {
                let pulse_tick = self.session.as_ref().and_then(|s| s.battle()).map(|game| {
                    let tick = game.world.tick;
                    self.local.select_same_type(game);
                    tracing::info!("同类型选中 · {} 个 · {:?}", self.local.selected.len(), self.local.selected);
                    tick
                });
                if let Some(tick) = pulse_tick {
                    self.pulse_action_lines_at(tick);
                }
            }
            "Team01" => self.handle_control_team(0),
            "Team02" => self.handle_control_team(1),
            "PlanningMode" => {
                if self.planning_mode {
                    self.commit_planning_waypoints();
                }
                else {
                    self.planning_mode = true;
                    self.planning_waypoints.clear();
                    self.place_mode = None;
                    self.repair_mode = false;
                    self.sell_mode = false;
                    tracing::info!(active = true, "命令条 · 路径点规划");
                }
            }
            _ => {
                // 其它命令条槽随后续对局命令接线补齐。
            }
        }
    }

    /// 命令条编队：`Ctrl` 写入当前选中，否则召回。
    pub(super) fn handle_control_team(&mut self, slot: usize) {
        let pulse_tick = self.session.as_ref().and_then(|s| s.battle()).map(|game| {
            let tick = game.world.tick;
            if self.ctrl_down {
                self.local.assign_team(game, slot);
                tracing::info!(slot = slot + 1, count = self.local.selected.len(), "编队 · 写入 Team{:02}", slot + 1);
            }
            else {
                let n = self.local.recall_team(game, slot);
                tracing::info!(slot = slot + 1, count = n, "编队 · 召回 Team{:02}", slot + 1);
            }
            tick
        });
        if let Some(tick) = pulse_tick {
            self.pulse_action_lines_at(tick);
        }
    }
}
