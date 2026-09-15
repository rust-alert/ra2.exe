//! 对局页控制器：输入意图、命令、tick、快照；不含窗口与页面导航外壳。

use ra_layout::{cameo_visible_slot_count, rect_px_from_snapshot};
use ra_map::{MapEntityKind, iso_to_screen};
use ra_renderer::Renderer;
use ra_types::EntityId;
use ra_widgets::battle_hud::BattleHudHit;
use winit::{
    event::{ElementState, MouseButton, MouseScrollDelta, WindowEvent},
    keyboard::{KeyCode, PhysicalKey},
    window::Window,
};

use super::super::battle_input::{
    LeftGesture, LeftReleaseAction, MARQUEE_HIT_HALF_INFANTRY_PX, MARQUEE_HIT_HALF_VEHICLE_PX, MARQUEE_VEHICLE_LIFT_PX, ScreenRect,
};

use super::{BattleController, BattleNav};

impl BattleController {
    /// 把 `Session::tick_fraction` 写入战斗会话，供点选 / 框选与烤图同一滑移脚点。
    pub(super) fn sync_present_tick_fraction(&mut self) {
        let tick_fraction = self.session.as_ref().map(|s| s.tick_fraction()).unwrap_or(0.0);
        if let Some(game) = self.session.as_mut().and_then(|s| s.battle_mut()) {
            game.present_tick_fraction = tick_fraction;
        }
    }

    /// 对局指针：边缘滚屏优先，否则按悬停格给出 Select / Move / Attack 等。
    pub fn battle_pointer(&self, renderer: &Renderer, window: &Window) -> super::super::battle_input::BattlePointer {
        use super::super::battle_input::BattlePointer;
        let context = self.battle_pointer_context(renderer, window);
        BattlePointer::resolve(self.edge_scroll_cursor, context)
    }

    /// 战术区悬停上下文（不含边缘滚屏）。
    ///
    /// 西木口径：悬停**已选**可部署单位显示 Deploy；`D` / 命令条立即下发，无单独部署工具态。
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

        // 出售工具：专用 Sell 光标（右键取消工具态）。
        if self.sell_mode {
            return BattlePointer::Sell;
        }
        // 修理工具：专用 Repair 光标（右键取消工具态）。
        if self.repair_mode {
            return BattlePointer::Repair;
        }
        // 建造放置：不伪装成可下令 Move（右键只取消放置）。
        if self.place_mode.is_some() {
            return BattlePointer::Default;
        }

        // 跟随模式：悬停任意存活机动单位时用点选光标（右键取消）。
        if self.follow_mode {
            if game.pick_any_mobile_near_image(wx, wy, 72.0).is_some() {
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

        let has_mobile = selected.iter().any(|&id| {
            game.world
                .ecs_identity(id)
                .is_some_and(|(_, kind)| matches!(kind, MapEntityKind::Unit | MapEntityKind::Infantry | MapEntityKind::Aircraft))
        });
        let selected_naval_only = has_mobile
            && selected.iter().filter(|&&id| {
                game.world
                    .ecs_identity(id)
                    .is_some_and(|(_, kind)| matches!(kind, MapEntityKind::Unit | MapEntityKind::Infantry | MapEntityKind::Aircraft))
            }).all(|&id| game.world.entity_is_naval(id));

        // 悬停已选可部署单位 → Deploy（先于攻击 / 移动，避免被友军格 Move 盖住）。
        if let Some(id) = game.pick_local_mobile_near_image(wx, wy, 72.0) {
            if selected.contains(&id) && game.entity_can_deploy(id) {
                return BattlePointer::Deploy;
            }
        }

        // 悬停已选生产厂 → Select（再点左键设 PRI；空地仍走下方 Move 设集结）。
        if let Some(id) = Self::pick_local_building_at_image(game, wx, wy) {
            if selected.contains(&id) && game.selection_has_primary_factory(&[id]) {
                return BattlePointer::Select;
            }
        }

        // 攻击移动模式：空地与敌方均显示攻击光标（右键取消模式）。
        if self.attack_move_mode && has_mobile {
            if game.pick_hostile_near_image(wx, wy, 72.0).is_some() {
                return BattlePointer::Attack;
            }
            let ok = game.world.pass_grid.in_bounds(cell.0, cell.1)
                && game.world.pass_grid.is_traversable(cell.0, cell.1, selected_naval_only);
            return if ok { BattlePointer::Attack } else { BattlePointer::NoMove };
        }

        // 已选机动单位时：异阵营目标用图像软命中（与左键攻击同口径）。
        if has_mobile && game.pick_hostile_near_image(wx, wy, 72.0).is_some() {
            return BattlePointer::Attack;
        }

        let ok = game.world.pass_grid.in_bounds(cell.0, cell.1)
            && game.world.pass_grid.is_traversable(cell.0, cell.1, selected_naval_only);
        if ok { BattlePointer::Move } else { BattlePointer::NoMove }
    }

    /// 可玩对局且未暂停 / 未结算时，壳层应捕获光标以支持边缘滚屏。
    pub fn wants_cursor_capture(&self) -> bool {
        self.session
            .as_ref()
            .and_then(|s| s.battle())
            .is_some_and(|g| !g.paused && g.outcome.is_none() && !g.world.trigger_runtime.script_input_locked)
    }

    pub(super) fn handle_left_click(&mut self, renderer: &Renderer, window: &Window) {
        self.sync_present_tick_fraction();
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
            let Some(sdef) = game.world.definitions.structures.get(&type_id)
            else {
                return;
            };
            let house = game.world.players.iter().find(|p| p.id == game.world.local_player).map(|p| p.house.clone());
            let Some(house) = house
            else {
                return;
            };
            if !game.world.can_place_building_for(house.as_ref(), sdef.id, cell.0, cell.1) {
                tracing::debug!("放置跳过 · 占地或建区不可用 {type_id} @({},{})", cell.0, cell.1);
                return;
            }
            if let Some(game) = self.session.as_mut().and_then(|s| s.battle_mut()) {
                tracing::info!("放置建筑 {type_id} @({},{})", cell.0, cell.1);
                game.order_place_building(type_id.clone(), cell.0, cell.1);
                // `PlaceBuilding` 入队后下一拍才消费完工件，此处 `is_local_ready_to_place` 仍为 true。
                // 占地已在主机侧校验。退出放置由 `sync_place_mode_with_ready` 在仿真推进后对齐。
                // 若命令被拒，完工件仍在，放置态保持，便于继续点合法格。
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
        // 路径点规划：左键追加航点（右键只负责取消）。
        if self.planning_mode {
            let Some(cell) = game.image_to_cell(wx, wy)
            else {
                return;
            };
            if self.local.selected.is_empty() {
                tracing::info!("路径点规划 · 无选中单位，忽略航点");
                return;
            }
            if self.planning_waypoints.last().copied() != Some(cell) {
                self.planning_waypoints.push(cell);
            }
            tracing::info!(count = self.planning_waypoints.len(), x = cell.0, y = cell.1, "路径点规划 · 追加航点");
            return;
        }
        // 跟随模式：左键点选任意机动单位作为跟随目标。
        if self.follow_mode {
            let selected = self.local.selected.clone();
            let has_mobile = selected.iter().any(|&id| {
                game.world
                    .ecs_identity(id)
                    .is_some_and(|(_, kind)| matches!(kind, MapEntityKind::Unit | MapEntityKind::Infantry | MapEntityKind::Aircraft))
            });
            if selected.is_empty() || !has_mobile {
                self.follow_mode = false;
                tracing::info!(active = false, "跟随模式 · 选中无效，已退出");
                return;
            }
            let tick = game.world.tick;
            if let Some(target) = game.pick_any_mobile_near_image(wx, wy, 72.0) {
                if selected.contains(&target) {
                    tracing::info!("跟随 · 目标在当前选中内，忽略");
                    return;
                }
                if let Some(game) = self.session.as_mut().and_then(|s| s.battle_mut()) {
                    tracing::info!("命令跟随 → #{}（选中 {:?}）", target.0, selected);
                    game.order_follow(&selected, target);
                }
                self.follow_mode = false;
                self.pulse_action_lines_at(tick);
                return;
            }
            tracing::info!("跟随 · 未命中机动单位");
            return;
        }

        let local_house = game.world.players.iter().find(|p| p.id == game.world.local_player).map(|p| p.house.to_string());
        let tick = game.world.tick;
        let selected = self.local.selected.clone();
        let has_mobile = selected.iter().any(|&id| {
            game.world
                .ecs_identity(id)
                .is_some_and(|(_, kind)| matches!(kind, MapEntityKind::Unit | MapEntityKind::Infantry | MapEntityKind::Aircraft))
        });
        let has_structure = game.selection_has_structure(&selected);
        let order_mod = super::super::battle_input::OrderClickModifier::from_keys(self.ctrl_down, self.alt_down);
        let queue_path = self.shift_down;

        // 本方单位 / 建筑优先：选择（或加选），不发移动 / 攻击。
        // Alt 强制移动时跳过友军点选，允许点到友军所占格仍下令移动。
        let skip_friendly_pick = matches!(order_mod, super::super::battle_input::OrderClickModifier::ForceMove) && has_mobile;
        if !skip_friendly_pick {
            let local_picked =
                game.pick_local_mobile_near_image(wx, wy, 72.0).or_else(|| Self::pick_local_building_at_image(game, wx, wy)).or_else(|| {
                    let cell = game.image_to_cell(wx, wy)?;
                    if let Some(house) = local_house.as_deref() {
                        game.pick_mobile_at_owned(cell.0, cell.1, Some(house))
                    }
                    else {
                        game.pick_mobile_at(cell.0, cell.1)
                    }
                });
            if let Some(id) = local_picked {
                let cell = game.world.ecs_transform(id).map(|(x, y, _)| (x, y)).unwrap_or((0, 0));
                // 西木：左键点已选可部署单位 → 立即部署（非 Shift 加选）。
                if !add && self.local.selected.contains(&id) && game.entity_can_deploy(id) {
                    let tick = game.world.tick;
                    self.deploy_selection();
                    self.pulse_action_lines_at(tick);
                    return;
                }
                // 再点已选生产厂 → 设为主厂（PRI）。
                if !add && self.local.selected.contains(&id) && game.selection_has_primary_factory(&[id]) {
                    if let Some(game) = self.session.as_mut().and_then(|s| s.battle_mut()) {
                        tracing::info!("设为主厂 → #{}（选中 {:?}）", id.0, selected);
                        game.order_set_primary(&[id]);
                    }
                    self.pulse_action_lines_at(tick);
                    return;
                }
                if add {
                    self.local.select_add(game, id);
                    tracing::info!("加选实体 #{} @({},{}) · 选中 {:?}", id.0, cell.0, cell.1, self.local.selected);
                }
                else {
                    self.local.select_only(game, id);
                    tracing::info!("选中实体 #{} @({},{})", id.0, cell.0, cell.1);
                }
                self.pulse_action_lines_at(tick);
                return;
            }
        }

        // 已选机动单位：左键敌方 → 攻击 / 占领 / 渗透（Alt 强制移动则跳过，改走落点移动）。
        if has_mobile && !matches!(order_mod, super::super::battle_input::OrderClickModifier::ForceMove) {
            if let Some(target) = game.pick_hostile_near_image(wx, wy, 72.0) {
                let is_structure = game.world.ecs_identity(target).is_some_and(|(_, kind)| kind == MapEntityKind::Structure);
                let force_attack = matches!(order_mod, super::super::battle_input::OrderClickModifier::ForceAttack);
                if let Some(game) = self.session.as_mut().and_then(|s| s.battle_mut()) {
                    if !force_attack && is_structure && game.selection_has_engineer(&selected) && game.is_capturable_structure(target) {
                        tracing::info!("命令占领 → #{}（选中 {:?}）", target.0, selected);
                        game.order_capture_building(&selected, target);
                    }
                    else if !force_attack && is_structure && game.selection_has_agent(&selected) {
                        tracing::info!("命令渗透 → #{}（选中 {:?}）", target.0, selected);
                        game.order_infiltrate(&selected, target);
                    }
                    else {
                        tracing::info!(force = force_attack, "命令攻击 → #{}（选中 {:?}）", target.0, selected);
                        game.order_attack(&selected, target);
                    }
                }
                self.attack_move_mode = false;
                self.follow_mode = false;
                self.pulse_action_lines_at(tick);
                return;
            }
        }

        // 已选单位 / 建筑：左键空地 → 移动、攻击移动、强制攻击近似或设集结点。
        if let Some(cell) = game.image_to_cell(wx, wy) {
            if has_mobile {
                if queue_path && matches!(order_mod, super::super::battle_input::OrderClickModifier::None) && !self.attack_move_mode {
                    // Shift+左键空地：追加路径点并下发整条路径。
                    if self.planning_waypoints.last().copied() != Some(cell) {
                        self.planning_waypoints.push(cell);
                    }
                    let points = self.planning_waypoints.clone();
                    let pulse_tick = self.session.as_mut().and_then(|s| s.battle_mut()).map(|game| {
                        let tick = game.world.tick;
                        tracing::info!(count = points.len(), "Shift 路径 · 下发 MovePath → ({},{})", cell.0, cell.1);
                        game.order_move_path(&selected, &points);
                        tick
                    });
                    if let Some(tick) = pulse_tick {
                        self.pulse_action_lines_at(tick);
                    }
                    return;
                }
                if let Some(game) = self.session.as_mut().and_then(|s| s.battle_mut()) {
                    if matches!(order_mod, super::super::battle_input::OrderClickModifier::ForceAttack) || self.attack_move_mode {
                        tracing::info!("命令攻击移动 → ({},{})（选中 {:?}）", cell.0, cell.1, selected);
                        game.order_attack_move(&selected, cell.0, cell.1);
                    }
                    else {
                        tracing::info!("命令移动 → ({},{})（选中 {:?}）", cell.0, cell.1, selected);
                        game.order_move(&selected, cell.0, cell.1);
                    }
                }
                if !queue_path {
                    self.planning_waypoints.clear();
                }
                self.attack_move_mode = false;
                self.follow_mode = false;
                self.pulse_action_lines_at(tick);
                return;
            }
            if has_structure {
                if let Some(game) = self.session.as_mut().and_then(|s| s.battle_mut()) {
                    tracing::info!("设置集结点 → ({},{})（选中 {:?}）", cell.0, cell.1, selected);
                    game.order_rally(&selected, cell.0, cell.1);
                }
                self.pulse_action_lines_at(tick);
                return;
            }
            if !add {
                self.local.clear();
                tracing::debug!("点空地 ({},{})，清空选中", cell.0, cell.1);
            }
            return;
        }

        if !add && !selected.is_empty() {
            self.local.clear();
        }
    }

    /// 框选：按实体屏幕包围盒与拖拽矩形相交，选中本方可控移动单位。
    pub(super) fn handle_marquee_select(&mut self, renderer: &Renderer, window: &Window, rect: ScreenRect) {
        self.sync_present_tick_fraction();
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
            let (fx, fy) = game.mobile_foot_pixel_offset_for(id);
            // 与标记 / `pick_local_mobile_near_image` 同一脚点锚；载具再上移以覆盖 VXL 车身。
            let wx = (sx - game.preview_origin_x + fx) as f32 + 30.0;
            let wy = (sy - game.preview_origin_y + fy) as f32 + 15.0;
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
        self.sync_present_tick_fraction();
        let _ = renderer;
        // 西木右键：先取消工具态（保留选中），不是停止，也不是下令。
        if self.clear_sidebar_tool_modes() {
            return;
        }
        // 单位/建筑 cameo：右键取消该类型在产或候补一件。
        let x = self.cursor.0 as i32;
        let y = self.cursor.1 as i32;
        if let Some(BattleHudHit::Cameo(slot)) = self.hit_hud_at(window, x, y) {
            if let Some(caps) = self.current_capabilities() {
                let items = Self::tab_items(&caps, self.sidebar_tab);
                let index = self.cameo_scroll.saturating_add(slot);
                if let Some(item) = items.get(index) {
                    let type_id = item.type_id.as_ref().to_string();
                    if let Some(game) = self.session.as_mut().and_then(|s| s.battle_mut()) {
                        if game.is_local_producing(&type_id) {
                            tracing::info!("取消生产 · {type_id}");
                            game.order_cancel_produce(type_id);
                            return;
                        }
                    }
                }
            }
        }
        // 无工具态：清空选中，回到默认箭头（停止只走 Stop / `S`）。
        if !self.local.selected.is_empty() {
            tracing::info!("右键 · 清空选中 {} 个", self.local.selected.len());
            self.local.clear();
        }
    }

    /// 对局页输入。`accept_commands=false`（结算）时仅允许确认离开 / 战役下一关。
    pub fn handle_event(&mut self, event: &WindowEvent, renderer: &mut Renderer, window: &Window, accept_commands: bool) -> BattleNav {
        let battle_paused = self.session.as_ref().and_then(|s| s.battle()).is_some_and(|g| g.paused);
        let script_locked = self.session.as_ref().and_then(|s| s.battle()).is_some_and(|g| g.world.trigger_runtime.script_input_locked);
        let gameplay_open = accept_commands && !battle_paused && !script_locked;
        let outcome_hold =
            self.session.as_ref().and_then(|s| s.battle()).is_some_and(|g| g.outcome.is_some() || g.pending_savour_outcome.is_some());
        // 收束窗 / EVA 播报：仍在 Battle 页，但不再接受对局/暂停输入。
        if accept_commands && outcome_hold {
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
            WindowEvent::MouseInput { state, button: MouseButton::Left, .. } if gameplay_open => {
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
                                | BattleHudHit::Diplomacy
                                | BattleHudHit::Radar),
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
                                }
                                else {
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
                        }
                        else if let Some(hit) = pressed_side {
                            let x = self.cursor.0 as i32;
                            let y = self.cursor.1 as i32;
                            if self.hit_hud_at(window, x, y) == Some(hit) {
                                if hit == BattleHudHit::Radar {
                                    self.focus_radar_click(renderer, window, x, y);
                                }
                                else {
                                    nav = self.on_sidebar_hit(hit);
                                }
                            }
                            self.left_gesture = LeftGesture::Idle;
                        }
                        else {
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
            WindowEvent::MouseInput { state: ElementState::Pressed, button: MouseButton::Right, .. } if gameplay_open => {
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
                    self.handle_pause_layer_drag(window);
                }
                else if script_locked {
                    self.left_gesture = LeftGesture::Idle;
                    self.camera_pan_keys.clear();
                }
                else {
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
                if gameplay_open && self.cursor_over_cameo_band(window) {
                    let steps = match delta {
                        MouseScrollDelta::LineDelta(_, y) => {
                            if *y > 0.0 {
                                -1
                            }
                            else if *y < 0.0 {
                                1
                            }
                            else {
                                0
                            }
                        }
                        MouseScrollDelta::PixelDelta(p) => {
                            if p.y > 0.0 {
                                -1
                            }
                            else if p.y < 0.0 {
                                1
                            }
                            else {
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
                    if !gameplay_open {
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
                            }
                            else {
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

                // 暂停中：Esc / Options 热键走暂停子层路由（选项回 Menu，确认/主菜单恢复对局）。
                if battle_paused {
                    if matches!(hotkey, Some(super::super::battle_hotkeys::HotkeyAction::Options)) {
                        return self.handle_pause_layer_escape();
                    }
                    return BattleNav::None;
                }

                // 剧本锁输入：仍允许 Options/Esc 进暂停，其它对局热键吞掉。
                if script_locked {
                    if matches!(hotkey, Some(super::super::battle_hotkeys::HotkeyAction::Options)) {
                        return self.dispatch_hotkey_action(super::super::battle_hotkeys::HotkeyAction::Options, renderer, window);
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
                    if !game.paused {
                        game.toggle_pause();
                    }
                    self.open_pause_menu_layer();
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
                if let Some(game) = self.session.as_ref().and_then(|s| s.battle()) {
                    let house = game.world.players.iter().find(|p| p.id == game.world.local_player).map(|p| p.house.to_string());
                    if let Some(house) = house {
                        if let Some((x, y)) = game.world.last_radar_event_cell(&house) {
                            self.focus_camera_on_cell(renderer, x, y);
                            tracing::info!(x, y, "CenterOnRadarEvent");
                        }
                    }
                }
                BattleNav::None
            }
            HotkeyAction::DeployObject => {
                let tick = self.session.as_ref().and_then(|s| s.battle()).map(|g| g.world.tick);
                self.deploy_selection();
                if let Some(tick) = tick {
                    self.pulse_action_lines_at(tick);
                }
                BattleNav::None
            }
            HotkeyAction::GuardObject => {
                self.guard_selection();
                BattleNav::None
            }
            HotkeyAction::StopObject => {
                self.stop_selection();
                BattleNav::None
            }
            HotkeyAction::AttackMove => {
                self.toggle_attack_move_mode();
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
                }
                else {
                    self.planning_mode = true;
                    self.planning_waypoints.clear();
                    self.place_mode = None;
                    self.repair_mode = false;
                    self.sell_mode = false;
                    tracing::info!(active = true, "PlanningMode");
                }
                BattleNav::None
            }
            HotkeyAction::MakePrimary => {
                let selected = self.local.selected.clone();
                if selected.is_empty() {
                    tracing::info!("设为主厂 · 无选中");
                    return BattleNav::None;
                }
                let pulse_tick = self.session.as_mut().and_then(|s| s.battle_mut()).map(|game| {
                    let tick = game.world.tick;
                    if game.selection_has_primary_factory(&selected) {
                        tracing::info!("设为主厂 · {:?}", selected);
                        game.order_set_primary(&selected);
                    }
                    else {
                        tracing::info!("设为主厂 · 选中无生产厂 {:?}", selected);
                    }
                    tick
                });
                if let Some(tick) = pulse_tick {
                    self.pulse_action_lines_at(tick);
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
                    }
                    else {
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
            HotkeyAction::ScatterObject => {
                self.scatter_selection();
                BattleNav::None
            }
            HotkeyAction::Delete => {
                self.delete_selection();
                BattleNav::None
            }
            HotkeyAction::Follow => {
                self.toggle_follow_mode();
                BattleNav::None
            }
            HotkeyAction::ToggleAlliance
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
