//! 对局页控制器：输入意图、命令、tick、快照；不含窗口与页面导航外壳。

use std::{
    collections::{HashMap, HashSet},
    path::PathBuf,
    sync::Arc,
    time::{Duration, Instant},
};

use ra_adaptor::RulesSystem;
use ra_assets::{CsfFile, FntFile, IniDocument, Rgba, tiberium_overlay_display_hsv};
use ra_engine::{
    BattleCapabilitiesSnapshot, BattleOutcome, CELL_MOVE_COST, CapabilityItem, Engine, HudSnapshot, Session, SessionPhase,
    terrain_spawner_frame_signature,
};
use ra_layout::{
    BattleHudChromeMetrics, MapViewport, SIDEBAR_TAB_COUNT, cameo_visible_slot_count, rect_px_from_snapshot, solve_battle_hud_with_metrics,
};
use ra_map::{
    MapEntity, MapEntityKind, MobilePaintPose, OverlayLayerFilter, StructureAnimBank, StructureBuildupClip, TILE_HEIGHT, TILE_WIDTH,
    TerrainAnimBank, Theater, WeatherParticleField, collect_structure_anim_bank, iso_to_screen, load_structure_buildup_clip,
    local_size_preview_rect, paint_mobiles_onto_preview_rgba, paint_ore_tree_frames_onto_rgba, paint_overlays_onto_preview_rgba,
    paint_structure_anims_onto_rgba, paint_structure_buildup_onto_rgba, paint_structures_onto_rgba, paint_terrain_anims_onto_rgba,
};
use ra_renderer::{Renderer, RgbaImage};
use ra_types::{EntityId, PresentFeel};
use ra_widgets::{
    battle_hud::{BattleCameoPaint, BattleHudChrome, BattleHudHit, decode_battle_hud_chrome_with, decode_cameo_sprite, hit_at_with_chrome},
    battle_order_icons::load_battle_order_icons,
    battle_pause_menu::{self, BattlePauseChrome, BattlePauseMenuHit},
    battle_selection_overlay::load_selection_overlay,
    compose::{BattleHudModel, blit_rgba, compose_battle_hud_overlay, compose_battle_pause_menu_overlay},
    fs_source::GameAssetSource,
    render::present,
    skin::{
        decode::DecodedUiSprite,
        text::{command_button_csf_tooltip, resolve_csf_text},
    },
};
use winit::{
    event::{ElementState, MouseButton, MouseScrollDelta, WindowEvent},
    keyboard::{KeyCode, PhysicalKey}, 
    window::Window,
};

use super::super::{
    battle_input::{
        CameraPanKeys, EDGE_SCROLL_MARGIN_PX, EDGE_SCROLL_SPEED_PX_PER_SEC, EdgeScrollCursor, KEYBOARD_PAN_SPEED_PX_PER_SEC, LeftGesture,
        LeftReleaseAction, MARQUEE_HIT_HALF_INFANTRY_PX, MARQUEE_HIT_HALF_VEHICLE_PX, MARQUEE_VEHICLE_LIFT_PX, ScreenRect, edge_scroll_axes,
        edge_scroll_cursor_for, edge_scroll_screen_delta, keyboard_pan_screen_delta,
    },
    boot::{BootResult, remap_owner_palette},
    local_player::LocalPlayerController,
};

use super::{BattleController, BattleNav};


impl BattleController {
    /// 对局指针：边缘滚屏优先，否则按悬停格给出 Select / Move / Attack / Deploy 等。
    pub fn battle_pointer(&self, renderer: &Renderer, window: &Window) -> super::super::battle_input::BattlePointer {
        use super::super::battle_input::BattlePointer;
        let context = self.battle_pointer_context(renderer, window);
        BattlePointer::resolve(self.edge_scroll_cursor, context)
    }

    /// 战术区悬停上下文（不含边缘滚屏）。
    ///
    /// 部署光标仅在悬停**已选中的可部署单位本身**时出现；移开即回到移动 / 攻击 / 默认。
    pub(super) fn battle_pointer_context(&self, renderer: &Renderer, window: &Window) -> super::super::battle_input::BattlePointer {
        use super::super::battle_input::BattlePointer;
        let Some(game) = self.session.as_ref().and_then(|s| s.battle())
        else {
            return BattlePointer::Default;
        };
        let selected = &self.local.selected;
        let Some(cell) = self.cursor_cell(renderer, window)
        else {
            return BattlePointer::Default;
        };
        let vp = self.map_viewport(window);
        let (wx, wy) = vp.screen_to_world(renderer.camera(), self.cursor.0 as f32, self.cursor.1 as f32);

        // 出售工具：悬停本方建筑时用点选光标提示可点售。
        if self.sell_mode {
            if Self::pick_local_building_at_image(game, wx, wy).is_some() {
                return BattlePointer::Select;
            }
            return BattlePointer::Default;
        }
        // 修理工具：悬停本方建筑时用点选光标提示可点修。
        if self.repair_mode {
            if Self::pick_local_building_at_image(game, wx, wy).is_some() {
                return BattlePointer::Select;
            }
            return BattlePointer::Default;
        }

        if selected.is_empty() {
            if game.pick_local_mobile_near_image(wx, wy, 72.0).is_some() || Self::pick_local_building_at_image(game, wx, wy).is_some() {
                return BattlePointer::Select;
            }
            return BattlePointer::Default;
        }

        // 悬停已选中的可部署单位 → 部署光标（移开则不再是部署）。
        if let Some(id) = game.pick_local_mobile_near_image(wx, wy, 72.0) {
            if selected.contains(&id) && game.deploy_target_of(id).is_some() {
                return BattlePointer::Deploy;
            }
        }

        if let Some(target) = game.pick_entity_at(cell.0, cell.1) {
            let hostile = selected.first().and_then(|&atk| {
                let a_owner = game.world.ecs_owner(atk)?;
                let t_owner = game.world.ecs_owner(target)?;
                Some(a_owner != t_owner)
            });
            if hostile == Some(true) {
                return BattlePointer::Attack;
            }
        }

        let passable = game.world.pass_grid.in_bounds(cell.0, cell.1) && game.world.pass_grid.is_passable(cell.0, cell.1);
        if passable { BattlePointer::Move } else { BattlePointer::NoMove }
    }

    /// 可玩对局且未暂停 / 未结算时，壳层应捕获光标以支持边缘滚屏。
    pub fn wants_cursor_capture(&self) -> bool {
        self.session.as_ref().and_then(|s| s.battle()).is_some_and(|g| !g.paused && g.outcome.is_none())
    }

    pub(super) fn handle_left_click(&mut self, renderer: &Renderer, window: &Window) {
        let add = self.shift_down;
        let vp = self.map_viewport(window);
        if !vp.contains_cursor(self.cursor.0 as i32, self.cursor.1 as i32) {
            if !add {
                self.local.clear();
            }
            return;
        }
        let Some(game) = self.session.as_ref().and_then(|s| s.battle())
        else {
            return;
        };
        let (wx, wy) = vp.screen_to_world(renderer.camera(), self.cursor.0 as f32, self.cursor.1 as f32);
        if let Some(type_id) = self.place_mode.clone() {
            let Some(cell) = game.image_to_cell(wx, wy)
            else {
                return;
            };
            let foundation = game.world.definitions.structures.get(&type_id).map(|s| s.foundation.clone()).unwrap_or_default();
            if !game.world.can_place_structure_footprint(cell.0, cell.1, foundation.width, foundation.height) {
                tracing::debug!("放置跳过 · 占地不可用 {type_id} @({},{})", cell.0, cell.1);
                return;
            }
            if let Some(game) = self.session.as_mut().and_then(|s| s.battle_mut()) {
                tracing::info!("放置建筑 {type_id} @({},{})", cell.0, cell.1);
                game.order_place_building(type_id.clone(), cell.0, cell.1);
                // 成功会清掉完工件；失败仍保持落位，便于接着点合法格。
                if !game.is_local_ready_to_place(&type_id) {
                    self.place_mode = None;
                }
            }
            return;
        }
        if self.sell_mode {
            // 出售：先占地格，再立面菱形（与点选同序，勿先软命中再漏格）。
            let building = Self::pick_local_building_at_image(game, wx, wy);
            if let Some(building) = building {
                if let Some(game) = self.session.as_mut().and_then(|s| s.battle_mut()) {
                    tracing::info!("出售建筑 · #{}", building.0);
                    game.order_sell_building(building);
                }
            }
            return;
        }
        if self.repair_mode {
            let building = Self::pick_local_building_at_image(game, wx, wy);
            if let Some(building) = building {
                if let Some(game) = self.session.as_mut().and_then(|s| s.battle_mut()) {
                    tracing::info!("修理建筑 · #{}", building.0);
                    game.order_repair_building(building);
                }
            }
            return;
        }
        let local_house = game.world.players.iter().find(|p| p.id == game.world.local_player).map(|p| p.house.to_string());
        let tick = game.world.tick;
        // 单位软命中优先（车身常偏格），再本方建筑占地格，再建筑立面菱形，最后格上单位。
        let picked =
            game.pick_local_mobile_near_image(wx, wy, 72.0).or_else(|| Self::pick_local_building_at_image(game, wx, wy)).or_else(|| {
                let cell = game.image_to_cell(wx, wy)?;
                if let Some(house) = local_house.as_deref() {
                    game.pick_mobile_at_owned(cell.0, cell.1, Some(house))
                } else {
                    game.pick_mobile_at(cell.0, cell.1)
                }
            });
        let mut pulse = false;
        if let Some(id) = picked {
            let cell = game.world.ecs_transform(id).map(|(x, y, _)| (x, y)).unwrap_or((0, 0));
            // 部署走 `D` / 命令条 Deploy，不在此用二次点击发明部署。
            if add {
                self.local.select_add(game, id);
                tracing::info!("加选实体 #{} @({},{}) · 选中 {:?}", id.0, cell.0, cell.1, self.local.selected);
            } else {
                self.local.select_only(game, id);
                tracing::info!("选中实体 #{} @({},{})", id.0, cell.0, cell.1);
            }
            pulse = true;
        } else if !add {
            self.local.clear();
            if let Some(cell) = game.image_to_cell(wx, wy) {
                tracing::debug!("点空地 ({},{})，清空选中", cell.0, cell.1);
            }
        }
        if pulse {
            self.pulse_action_lines_at(tick);
        }
    }

    /// 框选：按实体屏幕包围盒与拖拽矩形相交，选中本方可控移动单位。
    pub(super) fn handle_marquee_select(&mut self, renderer: &Renderer, window: &Window, rect: ScreenRect) {
        let add = self.shift_down;
        let Some(game) = self.session.as_ref().and_then(|s| s.battle())
        else {
            return;
        };
        let local_house = game.world.players.iter().find(|p| p.id == game.world.local_player).map(|p| p.house.to_string());
        let Some(house) = local_house.as_deref()
        else {
            return;
        };
        let vp = self.map_viewport(window);
        let cam = renderer.camera();
        let mut hits = Vec::new();
        for id in game.world.entity_ids() {
            if game.world.ecs_health(id).is_none_or(|(_, _, dead)| dead) {
                continue;
            }
            if game.world.ecs_owner(id).is_none_or(|o| o.as_ref() != house) {
                continue;
            }
            let Some((_, kind)) = game.world.ecs_identity(id)
            else {
                continue;
            };
            if !matches!(kind, MapEntityKind::Unit | MapEntityKind::Infantry | MapEntityKind::Aircraft) {
                continue;
            }
            let Some((x, y, _)) = game.world.ecs_transform(id)
            else {
                continue;
            };
            let z = game.world.pass_grid.cell_height(x, y);
            let (sx, sy) = iso_to_screen(i32::from(x), i32::from(y), z);
            // 与标记 / `pick_local_mobile_near_image` 同一脚点锚；载具再上移以覆盖 VXL 车身。
            let wx = (sx - game.preview_origin_x) as f32 + 30.0;
            let wy = (sy - game.preview_origin_y) as f32 + 15.0;
            let (cx, cy) = vp.world_to_screen(cam, wx, wy);
            let (hit_cx, hit_cy, half) = match kind {
                MapEntityKind::Infantry => (cx, cy, MARQUEE_HIT_HALF_INFANTRY_PX),
                MapEntityKind::Unit | MapEntityKind::Aircraft => (cx, cy - MARQUEE_VEHICLE_LIFT_PX, MARQUEE_HIT_HALF_VEHICLE_PX),
                _ => (cx, cy, MARQUEE_HIT_HALF_INFANTRY_PX),
            };
            let hit = ScreenRect::from_center_half(hit_cx, hit_cy, half);
            if rect.intersects(&hit) {
                hits.push(id);
            }
        }
        if hits.is_empty() {
            if !add {
                self.local.clear();
                tracing::debug!("框选落空，清空选中");
            }
            return;
        }
        self.local.apply_ids(game, &hits, add);
        let tick = game.world.tick;
        tracing::info!("框选命中 {} 个 · {:?}", hits.len(), self.local.selected);
        self.pulse_action_lines_at(tick);
    }

    pub(super) fn handle_right_click(&mut self, renderer: &Renderer, window: &Window) {
        if self.planning_mode {
            let Some(cell) = self.cursor_cell(renderer, window)
            else {
                return;
            };
            if self.local.selected.is_empty() {
                tracing::info!("路径点规划 · 无选中单位，忽略航点");
                return;
            };
            if self.planning_waypoints.last().copied() != Some(cell) {
                self.planning_waypoints.push(cell);
            }
            tracing::info!(count = self.planning_waypoints.len(), x = cell.0, y = cell.1, "路径点规划 · 追加航点");
            return;
        }
        if self.clear_sidebar_tool_modes() {
            return;
        }
        let Some(cell) = self.cursor_cell(renderer, window)
        else {
            return;
        };
        if self.local.selected.is_empty() {
            return;
        }
        let selected = self.local.selected.clone();
        let Some(game) = self.session.as_mut().and_then(|s| s.battle_mut())
        else {
            return;
        };
        if game.selection_has_structure(&selected) {
            tracing::info!("设置集结点 → ({},{})（选中 {:?}）", cell.0, cell.1, selected);
            game.order_rally(&selected, cell.0, cell.1);
            return;
        }
        let tick = game.world.tick;
        if let Some(target) = game.pick_entity_at(cell.0, cell.1) {
            let hostile = selected
                .first()
                .and_then(|&atk| {
                    let a_owner = game.world.ecs_owner(atk)?;
                    let t_owner = game.world.ecs_owner(target)?;
                    Some(a_owner != t_owner)
                })
                .unwrap_or(false);
            if hostile {
                let is_structure = game.world.ecs_identity(target).is_some_and(|(_, kind)| kind == MapEntityKind::Structure);
                if is_structure && game.selection_has_engineer(&selected) && game.is_capturable_structure(target) {
                    tracing::info!("命令占领 → #{}（选中 {:?}）", target.0, selected);
                    game.order_capture_building(&selected, target);
                } else if is_structure && game.selection_has_agent(&selected) {
                    tracing::info!("命令渗透 → #{}（选中 {:?}）", target.0, selected);
                    game.order_infiltrate(&selected, target);
                } else {
                    tracing::info!("命令攻击 → #{}（选中 {:?}）", target.0, selected);
                    game.order_attack(&selected, target);
                }
                self.pulse_action_lines_at(tick);
                return;
            }
        }
        tracing::info!("命令移动 → ({},{})（选中 {:?}）", cell.0, cell.1, selected);
        game.order_move(&selected, cell.0, cell.1);
        self.pulse_action_lines_at(tick);
    }

    /// 对局页输入。`accept_commands=false`（结算）时仅允许确认离开 / 战役下一关。
    pub fn handle_event(&mut self, event: &WindowEvent, renderer: &mut Renderer, window: &Window, accept_commands: bool) -> BattleNav {
        let battle_paused = self.session.as_ref().and_then(|s| s.battle()).is_some_and(|g| g.paused);
        let has_outcome = self.session.as_ref().and_then(|s| s.battle()).is_some_and(|g| g.outcome.is_some());
        // EVA 播报窗口：仍在 Battle 页，但不再接受对局/暂停输入。
        if accept_commands && has_outcome {
            if let WindowEvent::CursorMoved { position, .. } = event {
                self.cursor = (position.x, position.y);
            }
            return BattleNav::None;
        }
        match event {
            WindowEvent::ModifiersChanged(mods) => {
                self.shift_down = mods.state().shift_key();
                self.ctrl_down = mods.state().control_key();
                self.alt_down = mods.state().alt_key();
                BattleNav::None
            }
            WindowEvent::MouseInput { state, button: MouseButton::Left, .. } if accept_commands && battle_paused => {
                self.handle_pause_menu_mouse(*state, window)
            }
            WindowEvent::MouseInput { state, button: MouseButton::Left, .. } if accept_commands => {
                let mut nav = BattleNav::None;
                match state {
                    ElementState::Pressed => {
                        let x = self.cursor.0 as i32;
                        let y = self.cursor.1 as i32;
                        self.sidebar_pressed = None;
                        match self.hit_hud_at(window, x, y) {
                            Some(BattleHudHit::CommandButton(slot)) => {
                                self.command_pressed = Some(slot);
                                self.left_gesture = LeftGesture::Idle;
                            }
                            Some(
                                hit @ (BattleHudHit::SidebarTab(_)
                                | BattleHudHit::Cameo(_)
                                | BattleHudHit::Repair
                                | BattleHudHit::Sell
                                | BattleHudHit::Options
                                | BattleHudHit::Diplomacy),
                            ) => {
                                self.command_pressed = None;
                                self.sidebar_pressed = Some(hit);
                                self.left_gesture = LeftGesture::Idle;
                            }
                            None => {
                                self.command_pressed = None;
                                let vp = self.map_viewport(window);
                                if vp.contains_cursor(x, y) {
                                    self.left_gesture = LeftGesture::begin(self.cursor.0, self.cursor.1);
                                } else {
                                    self.left_gesture = LeftGesture::Idle;
                                }
                            }
                        }
                    }
                    ElementState::Released => {
                        let pressed_cmd = self.command_pressed.take();
                        let pressed_side = self.sidebar_pressed.take();
                        if let Some(slot) = pressed_cmd {
                            let x = self.cursor.0 as i32;
                            let y = self.cursor.1 as i32;
                            if matches!(
                                self.hit_hud_at(window, x, y),
                                Some(BattleHudHit::CommandButton(s)) if s == slot
                            ) {
                                self.on_command_button(slot);
                            }
                            self.left_gesture = LeftGesture::Idle;
                        } else if let Some(hit) = pressed_side {
                            let x = self.cursor.0 as i32;
                            let y = self.cursor.1 as i32;
                            if self.hit_hud_at(window, x, y) == Some(hit) {
                                nav = self.on_sidebar_hit(hit);
                            }
                            self.left_gesture = LeftGesture::Idle;
                        } else {
                            let (idle, action) = self.left_gesture.release();
                            self.left_gesture = idle;
                            match action {
                                LeftReleaseAction::None => {}
                                LeftReleaseAction::Click => self.handle_left_click(renderer, window),
                                LeftReleaseAction::Marquee(rect) => self.handle_marquee_select(renderer, window, rect),
                            }
                        }
                    }
                }
                nav
            }
            WindowEvent::MouseInput { state: ElementState::Released, button: MouseButton::Left, .. } if !accept_commands => {
                self.left_gesture = LeftGesture::Idle;
                self.command_pressed = None;
                self.sidebar_pressed = None;
                self.pause_pressed = None;
                BattleNav::None
            }
            WindowEvent::MouseInput { state: ElementState::Pressed, button: MouseButton::Right, .. } if accept_commands && !battle_paused => {
                self.left_gesture = LeftGesture::Idle;
                self.command_pressed = None;
                self.sidebar_pressed = None;
                self.handle_right_click(renderer, window);
                BattleNav::None
            }
            WindowEvent::CursorMoved { position, .. } => {
                self.cursor = (position.x, position.y);
                if battle_paused {
                    self.left_gesture = LeftGesture::Idle;
                    self.refresh_pause_hover(window);
                } else {
                    // 建造放置模式只认点选，拖拽不升为框选。
                    if accept_commands && self.place_mode.is_none() && self.command_pressed.is_none() && self.sidebar_pressed.is_none() {
                        self.left_gesture = self.left_gesture.on_cursor_moved(position.x, position.y);
                    }
                    self.refresh_command_hover(window);
                }
                BattleNav::None
            }
            WindowEvent::Focused(false) => {
                self.camera_pan_keys.clear();
                BattleNav::None
            }
            WindowEvent::MouseWheel { delta, .. } => {
                if accept_commands && !battle_paused && self.cursor_over_cameo_band(window) {
                    let steps = match delta {
                        MouseScrollDelta::LineDelta(_, y) => {
                            if *y > 0.0 {
                                -1
                            } else if *y < 0.0 {
                                1
                            } else {
                                0
                            }
                        }
                        MouseScrollDelta::PixelDelta(p) => {
                            if p.y > 0.0 {
                                -1
                            } else if p.y < 0.0 {
                                1
                            } else {
                                0
                            }
                        }
                    };
                    if steps != 0 {
                        self.scroll_cameos(window, steps);
                    }
                }
                BattleNav::None
            }
            WindowEvent::KeyboardInput { event, .. } => {
                let PhysicalKey::Code(code) = event.physical_key
                else {
                    return BattleNav::None;
                };
                let down = event.state == ElementState::Pressed;
                let vk = super::super::battle_hotkeys::key_code_to_vk(code);
                let hotkey = vk.and_then(|vk| self.hotkeys.action_for(vk, self.shift_down, self.ctrl_down, self.alt_down));

                // 方向键：未被 `keyboard.ini` 占用时才作镜头平移；侧栏箭头热键走查表。
                if matches!(code, KeyCode::ArrowLeft | KeyCode::ArrowRight | KeyCode::ArrowUp | KeyCode::ArrowDown) {
                    if !accept_commands || battle_paused {
                        self.camera_pan_keys.clear();
                        return BattleNav::None;
                    }
                    let claimed = hotkey.is_some();
                    if !claimed {
                        match code {
                            KeyCode::ArrowLeft => self.camera_pan_keys.left = down,
                            KeyCode::ArrowRight => self.camera_pan_keys.right = down,
                            KeyCode::ArrowUp => self.camera_pan_keys.up = down,
                            KeyCode::ArrowDown => self.camera_pan_keys.down = down,
                            _ => {}
                        }
                        return BattleNav::None;
                    }
                    if !down {
                        match code {
                            KeyCode::ArrowLeft => self.camera_pan_keys.left = false,
                            KeyCode::ArrowRight => self.camera_pan_keys.right = false,
                            KeyCode::ArrowUp => self.camera_pan_keys.up = false,
                            KeyCode::ArrowDown => self.camera_pan_keys.down = false,
                            _ => {}
                        }
                        return BattleNav::None;
                    }
                }

                if !down {
                    return BattleNav::None;
                }

                // 结算页：Enter / Esc 不属于 `[Hotkey]`。
                if !accept_commands {
                    return match code {
                        KeyCode::Enter | KeyCode::NumpadEnter => {
                            let continue_campaign = self.session.as_ref().and_then(|s| s.battle()).is_some_and(|g| {
                                if g.boot_kind != ra_engine::SessionBootKind::Campaign {
                                    return false;
                                }
                                match g.outcome.as_ref() {
                                    Some(ra_engine::BattleOutcome::Victory { .. }) => g.world.map.campaign_continue_scenario(true).is_some(),
                                    Some(ra_engine::BattleOutcome::Defeat { .. }) => g.world.map.campaign_continue_scenario(false).is_some(),
                                    None => false,
                                }
                            });
                            if continue_campaign {
                                tracing::info!("战役继续 · campaign continue scenario");
                                BattleNav::ContinueCampaign
                            } else {
                                tracing::info!("结算确认 · 离开");
                                BattleNav::ToMainMenu
                            }
                        }
                        KeyCode::Escape => {
                            tracing::info!("结算 · 离开");
                            BattleNav::ToMainMenu
                        }
                        _ => BattleNav::None,
                    };
                }

                // 暂停中：仅 `Options`（默认 Esc）关菜单。
                if battle_paused {
                    if matches!(hotkey, Some(super::super::battle_hotkeys::HotkeyAction::Options)) {
                        if let Some(game) = self.session.as_mut().and_then(|s| s.battle_mut()) {
                            game.toggle_pause();
                        }
                        self.clear_pause_menu_input();
                        tracing::info!("继续");
                    }
                    return BattleNav::None;
                }

                if let Some(action) = hotkey {
                    return self.dispatch_hotkey_action(action, renderer, window);
                }
                BattleNav::None
            }
            _ => BattleNav::None,
        }
    }

    /// 查表得到的 `HotkeyAction` 分发（键位来自 `keyboard.ini`，勿再写死 KeyCode）。
    pub(super) fn dispatch_hotkey_action(
        &mut self,
        action: super::super::battle_hotkeys::HotkeyAction,
        renderer: &mut Renderer,
        window: &Window,
    ) -> BattleNav {
        use super::super::battle_hotkeys::{HotkeyAction, team_slot_index};

        match action {
            HotkeyAction::Options => {
                if self.clear_sidebar_tool_modes() {
                    self.clear_pause_menu_input();
                    return BattleNav::None;
                }
                if let Some(game) = self.session.as_mut().and_then(|s| s.battle_mut()) {
                    game.toggle_pause();
                    self.clear_pause_menu_input();
                    tracing::info!("暂停菜单");
                }
                BattleNav::None
            }
            HotkeyAction::ScreenCapture => BattleNav::QueueScreenshot,
            HotkeyAction::CenterBase => {
                self.focus_camera_on_local_start(renderer);
                tracing::info!("CenterBase");
                BattleNav::None
            }
            HotkeyAction::CenterView => {
                if let Some(game) = self.session.as_ref().and_then(|s| s.battle()) {
                    if let Some((x, y)) = self.local.selection_focus_cell(game) {
                        self.focus_camera_on_cell(renderer, x, y);
                        tracing::info!(x, y, "CenterView");
                    }
                }
                BattleNav::None
            }
            HotkeyAction::CenterOnRadarEvent => {
                // 雷达事件未接前不发明其它行为。
                BattleNav::None
            }
            HotkeyAction::DeployObject => {
                self.deploy_selection();
                BattleNav::None
            }
            HotkeyAction::GuardObject => {
                self.guard_selection();
                BattleNav::None
            }
            HotkeyAction::CombatantSelect => {
                if let Some(game) = self.session.as_ref().and_then(|s| s.battle()) {
                    let seed = self.local.selected.first().copied().or_else(|| {
                        game.world.entity_ids().into_iter().find(|&eid| {
                            game.world.ecs_health(eid).is_some_and(|(_, _, dead)| !dead)
                                && game.world.ecs_identity(eid).is_some_and(|(_, kind)| {
                                matches!(kind, MapEntityKind::Unit | MapEntityKind::Infantry | MapEntityKind::Aircraft)
                            })
                        })
                    });
                    if let Some(id) = seed {
                        self.local.select_all_of_owner(game, id);
                        tracing::info!("CombatantSelect · {} 个", self.local.selected.len());
                    }
                }
                BattleNav::None
            }
            HotkeyAction::NextObject => {
                let pulse_tick = self.session.as_ref().and_then(|s| s.battle()).map(|game| {
                    let tick = game.world.tick;
                    self.local.cycle_selection(game);
                    tracing::info!("NextObject · {:?}", self.local.selected);
                    tick
                });
                if let Some(tick) = pulse_tick {
                    self.pulse_action_lines_at(tick);
                }
                BattleNav::None
            }
            HotkeyAction::TypeSelect => {
                let pulse_tick = self.session.as_ref().and_then(|s| s.battle()).map(|game| {
                    let tick = game.world.tick;
                    self.local.select_same_type(game);
                    tracing::info!("TypeSelect · {} 个", self.local.selected.len());
                    tick
                });
                if let Some(tick) = pulse_tick {
                    self.pulse_action_lines_at(tick);
                }
                BattleNav::None
            }
            HotkeyAction::StructureTab => {
                self.hotkey_sidebar_tab(0);
                BattleNav::None
            }
            HotkeyAction::DefenseTab => {
                self.hotkey_sidebar_tab(1);
                BattleNav::None
            }
            HotkeyAction::InfantryTab => {
                self.hotkey_sidebar_tab(2);
                BattleNav::None
            }
            HotkeyAction::UnitTab => {
                self.hotkey_sidebar_tab(3);
                BattleNav::None
            }
            HotkeyAction::ToggleRepair => {
                self.sell_mode = false;
                self.planning_mode = false;
                self.planning_waypoints.clear();
                self.repair_mode = !self.repair_mode;
                if self.repair_mode {
                    self.place_mode = None;
                }
                tracing::info!(active = self.repair_mode, "ToggleRepair");
                BattleNav::None
            }
            HotkeyAction::ToggleSell => {
                self.repair_mode = false;
                self.planning_mode = false;
                self.planning_waypoints.clear();
                self.sell_mode = !self.sell_mode;
                if self.sell_mode {
                    self.place_mode = None;
                }
                tracing::info!(active = self.sell_mode, "ToggleSell");
                BattleNav::None
            }
            HotkeyAction::PlanningMode => {
                if self.planning_mode {
                    self.commit_planning_waypoints();
                } else {
                    self.planning_mode = true;
                    self.planning_waypoints.clear();
                    self.place_mode = None;
                    self.repair_mode = false;
                    self.sell_mode = false;
                    tracing::info!(active = true, "PlanningMode");
                }
                BattleNav::None
            }
            HotkeyAction::LeftSidebarUp => {
                self.jump_cameo_scroll(window, false);
                BattleNav::None
            }
            HotkeyAction::LeftSidebarDown => {
                self.jump_cameo_scroll(window, true);
                BattleNav::None
            }
            HotkeyAction::RightSidebarUp | HotkeyAction::SidebarPageUp => {
                let snap = self.hud_snap_for_window(window);
                let page = cameo_visible_slot_count(rect_px_from_snapshot(&snap, "cameo_band").h).max(1) as i32;
                self.scroll_cameos(window, -page);
                BattleNav::None
            }
            HotkeyAction::RightSidebarDown | HotkeyAction::SidebarPageDown => {
                let snap = self.hud_snap_for_window(window);
                let page = cameo_visible_slot_count(rect_px_from_snapshot(&snap, "cameo_band").h).max(1) as i32;
                self.scroll_cameos(window, page);
                BattleNav::None
            }
            HotkeyAction::SidebarUp => {
                self.scroll_cameos(window, -2);
                BattleNav::None
            }
            HotkeyAction::SidebarDown => {
                self.scroll_cameos(window, 2);
                BattleNav::None
            }
            HotkeyAction::TeamSelect(n) => {
                if let Some(slot) = team_slot_index(n) {
                    let pulse_tick = self.session.as_ref().and_then(|s| s.battle()).map(|game| {
                        let tick = game.world.tick;
                        let count = self.local.recall_team(game, slot);
                        tracing::info!(slot = n, count, "TeamSelect");
                        tick
                    });
                    if let Some(tick) = pulse_tick {
                        self.pulse_action_lines_at(tick);
                    }
                }
                BattleNav::None
            }
            HotkeyAction::TeamCreate(n) => {
                if let Some(slot) = team_slot_index(n) {
                    let pulse_tick = self.session.as_ref().and_then(|s| s.battle()).map(|game| {
                        let tick = game.world.tick;
                        self.local.assign_team(game, slot);
                        tracing::info!(slot = n, count = self.local.selected.len(), "TeamCreate");
                        tick
                    });
                    if let Some(tick) = pulse_tick {
                        self.pulse_action_lines_at(tick);
                    }
                }
                BattleNav::None
            }
            HotkeyAction::TeamAddSelect(n) => {
                if let Some(slot) = team_slot_index(n) {
                    let pulse_tick = self.session.as_ref().and_then(|s| s.battle()).map(|game| {
                        let tick = game.world.tick;
                        let added = self.local.add_team_to_selection(game, slot);
                        tracing::info!(slot = n, added, "TeamAddSelect");
                        tick
                    });
                    if let Some(tick) = pulse_tick {
                        self.pulse_action_lines_at(tick);
                    }
                }
                BattleNav::None
            }
            HotkeyAction::TeamCenter(n) => {
                if let Some(slot) = team_slot_index(n) {
                    let cell = if let Some(game) = self.session.as_ref().and_then(|s| s.battle()) {
                        let _ = self.local.recall_team(game, slot);
                        self.local.team_focus_cell(game, slot)
                    } else {
                        None
                    };
                    if let Some((x, y)) = cell {
                        self.focus_camera_on_cell(renderer, x, y);
                        tracing::info!(slot = n, x, y, "TeamCenter");
                    }
                }
                BattleNav::None
            }
            HotkeyAction::SetView(n) => {
                self.set_view_bookmark(renderer, n);
                BattleNav::None
            }
            HotkeyAction::View(n) => {
                self.recall_view_bookmark(renderer, n);
                BattleNav::None
            }
            HotkeyAction::StopObject
            | HotkeyAction::ScatterObject
            | HotkeyAction::Follow
            | HotkeyAction::Delete
            | HotkeyAction::ToggleAlliance
            | HotkeyAction::PlaceBeacon
            | HotkeyAction::AllToCheer
            | HotkeyAction::PageUser
            | HotkeyAction::Taunt(_) => {
                tracing::debug!(?action, "热键已识别，能力未接，忽略");
                BattleNav::None
            }
        }
    }

    /// 关闭规划并下发暂存航点；无航点则仅退出规划。
    pub(super) fn commit_planning_waypoints(&mut self) {
        self.planning_mode = false;
        let points = std::mem::take(&mut self.planning_waypoints);
        if points.is_empty() {
            tracing::info!(active = false, "命令条 · 路径点规划（无航点）");
            return;
        }
        let selected = self.local.selected.clone();
        if selected.is_empty() {
            tracing::info!(active = false, count = points.len(), "命令条 · 路径点规划（无选中，已丢弃航点）");
            return;
        }
        let pulse_tick = self.session.as_mut().and_then(|s| s.battle_mut()).map(|game| {
            let tick = game.world.tick;
            tracing::info!(count = points.len(), units = selected.len(), "路径点规划 · 下发 MovePath");
            game.order_move_path(&selected, &points);
            tick
        });
        if let Some(tick) = pulse_tick {
            self.pulse_action_lines_at(tick);
        }
        tracing::info!(active = false, "命令条 · 路径点规划");
    }

    /// 本方建筑点选：先 `Foundation` 占地格，再立面等距菱形（与左键 / 出售 / 修理一致）。
    pub(super) fn pick_local_building_at_image(game: &ra_engine::BattleSession, wx: f32, wy: f32) -> Option<EntityId> {
        if let Some(cell) = game.image_to_cell(wx, wy) {
            let house = game.world.players.iter().find(|p| p.id == game.world.local_player).map(|p| p.house.as_ref())?;
            if let Some(id) = game.pick_structure_at(cell.0, cell.1).filter(|&id| game.world.ecs_owner(id).is_some_and(|o| o.as_ref() == house))
            {
                return Some(id);
            }
        }
        game.pick_local_structure_near_image(wx, wy)
    }
}
