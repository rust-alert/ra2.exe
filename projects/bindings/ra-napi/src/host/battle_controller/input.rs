//! 对局页控制器：输入意图、命令、tick、快照；不含窗口与页面导航外壳。

use super::super::battle_input::BattleInteractionMode;

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
    BattleUiCapture, LeftGesture, LeftReleaseAction, MARQUEE_HIT_HALF_INFANTRY_PX, MARQUEE_HIT_HALF_VEHICLE_PX, MARQUEE_VEHICLE_LIFT_PX,
    ScreenRect,
};

use super::{BattleController, BattleNav};

/// 战术区一次 soft-pick 探针（光标建议与左键命令共用同一组命中结果）。
#[derive(Debug, Clone, Copy)]
pub(super) struct BattleWorldProbe {
    /// 图像空间 X。
    pub world_x: f32,
    /// 图像空间 Y。
    pub world_y: f32,
    /// 光标下地图格。
    pub cell: Option<(u16, u16)>,
    /// 本方机动软命中（与 `pick_local_for_order` 软点选共用）。
    pub local_mobile: Option<EntityId>,
    /// 本方建筑命中（与 `pick_local_for_order` 软点选共用）。
    pub local_building: Option<EntityId>,
    /// 敌方软命中。
    pub hostile: Option<EntityId>,
    /// 任意阵营机动（跟随模式）。
    pub any_mobile: Option<EntityId>,
    /// 当前选中是否含机动单位。
    pub has_mobile_selected: bool,
    /// 当前选中是否含建筑。
    pub has_structure_selected: bool,
    /// 当前选中相对该格是否可通行。
    pub traversable: bool,
}

/// 战术区左键将执行的意图（由 `resolve_world_intent` 一次算出，光标与释放共用）。
#[derive(Debug, Clone)]
pub(super) enum WorldClickIntent {
    /// 无动作。
    Noop,
    /// 不可通行落点：光标 `NoMove`，左键不下令（与指针同源）。
    Blocked,
    /// 窗外 / 非战术区。
    OutsideWorld {
        /// 是否清空选中。
        clear_selection: bool,
    },
    /// 放置建筑。
    PlaceBuilding {
        type_id: String,
        cell: (u16, u16),
    },
    /// 出售。
    Sell(EntityId),
    /// 修理。
    Repair(EntityId),
    /// 规划航点。
    AppendWaypoint {
        cell: (u16, u16),
    },
    /// 跟随。
    Follow {
        target: EntityId,
    },
    /// 跟随模式选中无效 → 退出模式。
    FollowCancel,
    /// 点选 / 加选。
    Select {
        id: EntityId,
        add: bool,
    },
    /// 部署已选。
    Deploy,
    /// 设主厂。
    SetPrimary(EntityId),
    /// 攻击。
    Attack {
        target: EntityId,
    },
    /// 占领。
    Capture {
        target: EntityId,
    },
    /// 渗透。
    Infiltrate {
        target: EntityId,
    },
    /// 移动。
    Move {
        cell: (u16, u16),
        queue_path: bool,
    },
    /// 攻击移动。
    AttackMove {
        cell: (u16, u16),
    },
    /// Shift 路径。
    QueueMovePath {
        cell: (u16, u16),
    },
    /// 集结点。
    SetRally {
        cell: (u16, u16),
    },
    /// 清空选中。
    Deselect,
}

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
        let hover = self.resolve_battle_hover(renderer, window);
        BattlePointer::resolve(self.edge_scroll_cursor, hover.recommended_pointer)
    }

    /// 战术区一次解析：光标与左键命令共用同一意图。
    ///
    /// 西木口径：悬停**已选**可部署单位显示 Deploy；左键点该单位立即部署。
    pub(super) fn resolve_battle_hover(&self, renderer: &Renderer, window: &Window) -> super::super::battle_input::ResolvedBattleHover {
        let (hover, _intent) = self.resolve_world_intent(renderer, window);
        hover
    }

    /// 图像空间下的战场探针（一次 soft-pick，供光标与左键共用）。
    pub(super) fn probe_battle_world(&self, renderer: &Renderer, window: &Window) -> Option<BattleWorldProbe> {
        let game = self.session.as_ref()?.battle()?;
        let vp = self.map_viewport(window);
        if !vp.contains_cursor(self.cursor.0 as i32, self.cursor.1 as i32) {
            return None;
        }
        let (wx, wy) = vp.screen_to_world(renderer.camera(), self.cursor.0 as f32, self.cursor.1 as f32);
        let cell = game.image_to_cell(wx, wy);
        let selected = &self.local.selected;
        let has_mobile = selected.iter().any(|&id| {
            game.world
                .ecs_identity(id)
                .is_some_and(|(_, kind)| matches!(kind, MapEntityKind::Unit | MapEntityKind::Infantry | MapEntityKind::Aircraft))
        });
        let has_structure = game.selection_has_structure(selected);
        let selected_naval_only = has_mobile
            && selected
                .iter()
                .filter(|&&id| {
                    game.world
                        .ecs_identity(id)
                        .is_some_and(|(_, kind)| matches!(kind, MapEntityKind::Unit | MapEntityKind::Infantry | MapEntityKind::Aircraft))
                })
                .all(|&id| game.world.entity_is_naval(id));
        let traversable = cell
            .is_some_and(|(cx, cy)| game.world.pass_grid.in_bounds(cx, cy) && game.world.pass_grid.is_traversable(cx, cy, selected_naval_only));
        Some(BattleWorldProbe {
            world_x: wx,
            world_y: wy,
            cell,
            local_mobile: game.pick_local_mobile_near_image(wx, wy, 72.0),
            local_building: Self::pick_local_building_at_image(game, wx, wy),
            hostile: game.pick_hostile_near_image(wx, wy, 72.0),
            any_mobile: game.pick_any_mobile_near_image(wx, wy, 72.0),
            has_mobile_selected: has_mobile,
            has_structure_selected: has_structure,
            traversable,
        })
    }

    /// 按当前修饰键与交互模式，解析战术区左键意图（光标同源）。
    pub(super) fn resolve_world_intent(
        &self,
        renderer: &Renderer,
        window: &Window,
    ) -> (super::super::battle_input::ResolvedBattleHover, WorldClickIntent) {
        use super::super::battle_input::{OrderClickModifier, ResolvedBattleHover};

        let frame = self.input_frame(window);
        let add = frame.shift_down;
        let mode = &self.interaction_mode;
        let Some(probe) = self.probe_battle_world(renderer, window)
        else {
            let intent = WorldClickIntent::OutsideWorld { clear_selection: !add };
            return (Self::hover_from_intent(&intent, None, false, mode), intent);
        };
        let Some(game) = self.session.as_ref().and_then(|s| s.battle())
        else {
            return (ResolvedBattleHover::empty(), WorldClickIntent::Noop);
        };

        let order_mod = frame.order_mod;
        let queue_path = frame.shift_down;
        let selected = &self.local.selected;
        let finish = |intent: WorldClickIntent| (Self::hover_from_intent(&intent, probe.cell, probe.traversable, mode), intent);

        // —— 工具 / 命令模式（互斥）——
        if let Some(type_id) = mode.place_type_id() {
            let intent = match probe.cell {
                Some(cell) => {
                    let placeable = game
                        .world
                        .definitions
                        .structures
                        .get(type_id)
                        .and_then(|sdef| {
                            let house = game.world.players.iter().find(|p| p.id == game.world.local_player)?;
                            Some(game.world.can_place_building_for(house.house.as_ref(), sdef.id, cell.0, cell.1))
                        })
                        .unwrap_or(false);
                    if placeable {
                        WorldClickIntent::PlaceBuilding {
                            type_id: type_id.to_string(),
                            cell,
                        }
                    }
                    else {
                        WorldClickIntent::Noop
                    }
                }
                None => WorldClickIntent::Noop,
            };
            return finish(intent);
        }
        if mode.is_sell() {
            let intent = match probe.local_building {
                Some(id) => WorldClickIntent::Sell(id),
                None => WorldClickIntent::Noop,
            };
            return finish(intent);
        }
        if mode.is_repair() {
            let intent = match probe.local_building {
                Some(id) => WorldClickIntent::Repair(id),
                None => WorldClickIntent::Noop,
            };
            return finish(intent);
        }
        if mode.is_planning() {
            let intent = match probe.cell {
                Some(cell) if !selected.is_empty() && probe.traversable => WorldClickIntent::AppendWaypoint { cell },
                Some(_) if !selected.is_empty() => WorldClickIntent::Blocked,
                _ => WorldClickIntent::Noop,
            };
            return finish(intent);
        }
        if mode.is_follow() {
            let intent = if selected.is_empty() || !probe.has_mobile_selected {
                WorldClickIntent::FollowCancel
            }
            else if let Some(target) = probe.any_mobile {
                if selected.contains(&target) {
                    WorldClickIntent::Noop
                }
                else {
                    WorldClickIntent::Follow { target }
                }
            }
            else {
                WorldClickIntent::Noop
            };
            return finish(intent);
        }

        // —— 常规：先敌后友，再落点下令 ——
        if probe.has_mobile_selected && !matches!(order_mod, OrderClickModifier::ForceMove) {
            if let Some(target) = probe.hostile {
                let is_structure = game.world.ecs_identity(target).is_some_and(|(_, kind)| kind == MapEntityKind::Structure);
                let force_attack = matches!(order_mod, OrderClickModifier::ForceAttack);
                let intent = if !force_attack && is_structure && game.selection_has_engineer(selected) && game.is_capturable_structure(target)
                {
                    WorldClickIntent::Capture { target }
                }
                else if !force_attack && is_structure && game.selection_has_agent(selected) {
                    WorldClickIntent::Infiltrate { target }
                }
                else {
                    WorldClickIntent::Attack { target }
                };
                return finish(intent);
            }
        }

        let skip_friendly_pick = matches!(order_mod, OrderClickModifier::ForceMove) && probe.has_mobile_selected;
        if !skip_friendly_pick {
            if let Some(id) = self.pick_local_for_order(game, &probe) {
                let unit_cell = game.world.ecs_transform(id).map(|(x, y, _)| (x, y)).unwrap_or((0, 0));
                let on_structure_footprint = game.world.ecs_identity(id).is_some_and(|(_, kind)| kind == MapEntityKind::Structure)
                    && probe.cell.is_some_and(|(cx, cy)| game.pick_structure_at(cx, cy) == Some(id));
                let soft_hit_to_order = !on_structure_footprint
                    && super::super::battle_input::friendly_soft_hit_should_order_not_reselect(
                        probe.has_mobile_selected,
                        probe.cell,
                        Some(unit_cell),
                    );
                if soft_hit_to_order {
                    // 落入下方落点下令（与 Move 光标对齐）。
                }
                else if !add && selected.contains(&id) && game.entity_can_deploy(id) {
                    return finish(WorldClickIntent::Deploy);
                }
                else if !add && selected.contains(&id) && game.selection_has_primary_factory(&[id]) {
                    return finish(WorldClickIntent::SetPrimary(id));
                }
                else {
                    return finish(WorldClickIntent::Select { id, add });
                }
            }
        }

        if let Some(cell) = probe.cell {
            if probe.has_mobile_selected {
                let intent = if !probe.traversable {
                    // 不可通行：光标 NoMove，左键也不下令（避免「显示禁止却仍移动」）。
                    WorldClickIntent::Blocked
                }
                else if queue_path && matches!(order_mod, OrderClickModifier::None) && !mode.is_attack_move() {
                    WorldClickIntent::QueueMovePath { cell }
                }
                else if matches!(order_mod, OrderClickModifier::ForceAttack) || mode.is_attack_move() {
                    WorldClickIntent::AttackMove { cell }
                }
                else {
                    WorldClickIntent::Move {
                        cell,
                        queue_path,
                    }
                };
                return finish(intent);
            }
            if probe.has_structure_selected {
                return finish(WorldClickIntent::SetRally { cell });
            }
            let intent = if add {
                WorldClickIntent::Noop
            }
            else {
                WorldClickIntent::Deselect
            };
            return finish(intent);
        }

        let intent = if !add && !selected.is_empty() {
            WorldClickIntent::Deselect
        }
        else {
            WorldClickIntent::Noop
        };
        finish(intent)
    }

    /// 本方点选命中：有机动选中时只认落点格，避免邻矿被车身软命中吞掉。
    pub(super) fn pick_local_for_order(&self, game: &ra_engine::BattleSession, probe: &BattleWorldProbe) -> Option<EntityId> {
        let local_house = game.world.players.iter().find(|p| p.id == game.world.local_player).map(|p| p.house.to_string());
        let has_mobile = probe.has_mobile_selected;
        let (wx, wy) = (probe.world_x, probe.world_y);
        if super::super::battle_input::allow_friendly_image_soft_pick(has_mobile) {
            // 与悬停探针同一组 soft-pick，避免光标 / 点击各扫一次。
            probe
                .local_mobile
                .or(probe.local_building)
                .or_else(|| {
                    if !super::super::battle_input::allow_cell_neighbor_friendly_pick(has_mobile) {
                        return None;
                    }
                    let cell = game.image_to_cell(wx, wy)?;
                    if let Some(house) = local_house.as_deref() {
                        game.pick_mobile_at_owned(cell.0, cell.1, Some(house))
                    }
                    else {
                        game.pick_mobile_at(cell.0, cell.1)
                    }
                })
        }
        else {
            game.image_to_cell(wx, wy).and_then(|(cx, cy)| {
                let house = local_house.as_deref()?;
                game.pick_mobile_at_owned(cx, cy, Some(house))
                    .or_else(|| game.pick_structure_at(cx, cy).filter(|&id| game.world.ecs_owner(id).is_some_and(|o| o.as_ref() == house)))
            })
        }
    }

    /// 由意图推导悬停摘要（指针与主动作同源）。
    pub(super) fn hover_from_intent(
        intent: &WorldClickIntent,
        cell: Option<(u16, u16)>,
        traversable: bool,
        mode: &BattleInteractionMode,
    ) -> super::super::battle_input::ResolvedBattleHover {
        use super::super::battle_input::{BattlePointer, ResolvedBattleHover, ResolvedPrimaryAction};

        let (primary, recommended_pointer) = match intent {
            WorldClickIntent::Noop => {
                let pointer = if mode.is_sell() {
                    BattlePointer::Sell
                }
                else if mode.is_repair() {
                    BattlePointer::Repair
                }
                else if mode.place_type_id().is_some() {
                    BattlePointer::Default
                }
                else if mode.is_follow() {
                    BattlePointer::Default
                }
                else if mode.is_attack_move() {
                    if traversable { BattlePointer::Attack } else { BattlePointer::NoMove }
                }
                else {
                    BattlePointer::Default
                };
                (ResolvedPrimaryAction::Noop, pointer)
            }
            WorldClickIntent::Blocked => (ResolvedPrimaryAction::Noop, BattlePointer::NoMove),
            WorldClickIntent::OutsideWorld { clear_selection: true } => (ResolvedPrimaryAction::Deselect, BattlePointer::Default),
            WorldClickIntent::OutsideWorld { clear_selection: false } => (ResolvedPrimaryAction::Noop, BattlePointer::Default),
            WorldClickIntent::PlaceBuilding { .. } => (ResolvedPrimaryAction::PlaceBuilding, BattlePointer::Default),
            WorldClickIntent::Sell(_) => (ResolvedPrimaryAction::Sell, BattlePointer::Sell),
            WorldClickIntent::Repair(_) => (ResolvedPrimaryAction::Repair, BattlePointer::Repair),
            WorldClickIntent::AppendWaypoint { .. } => (
                ResolvedPrimaryAction::AppendWaypoint,
                if traversable { BattlePointer::Move } else { BattlePointer::NoMove },
            ),
            WorldClickIntent::Follow { .. } => (ResolvedPrimaryAction::Follow, BattlePointer::Select),
            WorldClickIntent::FollowCancel => (ResolvedPrimaryAction::Noop, BattlePointer::Default),
            WorldClickIntent::Select { add: true, .. } => (ResolvedPrimaryAction::AddSelect, BattlePointer::Select),
            WorldClickIntent::Select { add: false, .. } => (ResolvedPrimaryAction::Select, BattlePointer::Select),
            WorldClickIntent::Deploy => (ResolvedPrimaryAction::Deploy, BattlePointer::Deploy),
            WorldClickIntent::SetPrimary(_) => (ResolvedPrimaryAction::SetPrimary, BattlePointer::Select),
            WorldClickIntent::Attack { .. } => (ResolvedPrimaryAction::Attack, BattlePointer::Attack),
            WorldClickIntent::Capture { .. } => (ResolvedPrimaryAction::Capture, BattlePointer::Attack),
            WorldClickIntent::Infiltrate { .. } => (ResolvedPrimaryAction::Infiltrate, BattlePointer::Attack),
            WorldClickIntent::Move { .. } => (
                ResolvedPrimaryAction::Move,
                if traversable { BattlePointer::Move } else { BattlePointer::NoMove },
            ),
            WorldClickIntent::AttackMove { .. } => (
                ResolvedPrimaryAction::AttackMove,
                if traversable { BattlePointer::Attack } else { BattlePointer::NoMove },
            ),
            WorldClickIntent::QueueMovePath { .. } => (
                ResolvedPrimaryAction::QueueMovePath,
                if traversable { BattlePointer::Move } else { BattlePointer::NoMove },
            ),
            WorldClickIntent::SetRally { .. } => (ResolvedPrimaryAction::SetRally, BattlePointer::Move),
            WorldClickIntent::Deselect => (ResolvedPrimaryAction::Deselect, BattlePointer::Default),
        };

        ResolvedBattleHover {
            recommended_pointer,
            cell,
            primary,
        }
    }

    /// 可玩对局且未暂停 / 未结算时，壳层应捕获光标以支持边缘滚屏。
    pub fn wants_cursor_capture(&self) -> bool {
        self.session
            .as_ref()
            .and_then(|s| s.battle())
            .is_some_and(|g| !g.paused && g.outcome.is_none() && !g.world.trigger_runtime.script_input_locked)
    }

    /// 左键释放：解析一次意图并执行（与光标同源）。
    pub(super) fn handle_left_click(&mut self, renderer: &Renderer, window: &Window) {
        self.sync_present_tick_fraction();
        let (_hover, intent) = self.resolve_world_intent(renderer, window);
        self.apply_world_click_intent(intent);
    }

    /// 执行战术区左键意图。
    pub(super) fn apply_world_click_intent(&mut self, intent: WorldClickIntent) {
        match intent {
            WorldClickIntent::Noop | WorldClickIntent::Blocked => {}
            WorldClickIntent::OutsideWorld { clear_selection: false } => {}
            WorldClickIntent::OutsideWorld { clear_selection: true } | WorldClickIntent::Deselect => {
                self.local.clear();
            }
            WorldClickIntent::PlaceBuilding { type_id, cell } => {
                if let Some(game) = self.session.as_mut().and_then(|s| s.battle_mut()) {
                    tracing::info!("放置建筑 {type_id} @({},{})", cell.0, cell.1);
                    game.order_place_building(type_id, cell.0, cell.1);
                }
            }
            WorldClickIntent::Sell(building) => {
                if let Some(game) = self.session.as_mut().and_then(|s| s.battle_mut()) {
                    tracing::info!("出售建筑 · #{}", building.0);
                    game.order_sell_building(building);
                }
            }
            WorldClickIntent::Repair(building) => {
                if let Some(game) = self.session.as_mut().and_then(|s| s.battle_mut()) {
                    tracing::info!("修理建筑 · #{}", building.0);
                    game.order_repair_building(building);
                }
            }
            WorldClickIntent::AppendWaypoint { cell } => {
                if self.planning_waypoints.last().copied() != Some(cell) {
                    self.planning_waypoints.push(cell);
                }
                tracing::info!(count = self.planning_waypoints.len(), x = cell.0, y = cell.1, "路径点规划 · 追加航点");
            }
            WorldClickIntent::FollowCancel => {
                self.interaction_mode = BattleInteractionMode::Normal;
                tracing::info!(active = false, "跟随模式 · 选中无效，已退出");
            }
            WorldClickIntent::Follow { target } => {
                let selected = self.local.selected.clone();
                let tick = self.session.as_ref().and_then(|s| s.battle()).map(|g| g.world.tick);
                if let Some(game) = self.session.as_mut().and_then(|s| s.battle_mut()) {
                    tracing::info!("命令跟随 → #{}（选中 {:?}）", target.0, selected);
                    game.order_follow(&selected, target);
                }
                self.interaction_mode = BattleInteractionMode::Normal;
                if let Some(tick) = tick {
                    self.pulse_action_lines_at(tick);
                }
            }
            WorldClickIntent::Select { id, add } => {
                let Some(game) = self.session.as_ref().and_then(|s| s.battle())
                else {
                    return;
                };
                let unit_cell = game.world.ecs_transform(id).map(|(x, y, _)| (x, y)).unwrap_or((0, 0));
                let tick = game.world.tick;
                if add {
                    self.local.select_add(game, id);
                    tracing::info!("加选实体 #{} @({},{}) · 选中 {:?}", id.0, unit_cell.0, unit_cell.1, self.local.selected);
                }
                else {
                    self.local.select_only(game, id);
                    tracing::info!("选中实体 #{} @({},{})", id.0, unit_cell.0, unit_cell.1);
                }
                self.pulse_action_lines_at(tick);
            }
            WorldClickIntent::Deploy => {
                let tick = self.session.as_ref().and_then(|s| s.battle()).map(|g| g.world.tick);
                self.deploy_selection();
                if let Some(tick) = tick {
                    self.pulse_action_lines_at(tick);
                }
            }
            WorldClickIntent::SetPrimary(id) => {
                let selected = self.local.selected.clone();
                let tick = self.session.as_ref().and_then(|s| s.battle()).map(|g| g.world.tick);
                if let Some(game) = self.session.as_mut().and_then(|s| s.battle_mut()) {
                    tracing::info!("设为主厂 → #{}（选中 {:?}）", id.0, selected);
                    game.order_set_primary(&[id]);
                }
                if let Some(tick) = tick {
                    self.pulse_action_lines_at(tick);
                }
            }
            WorldClickIntent::Attack { target } => {
                let selected = self.local.selected.clone();
                let tick = self.session.as_ref().and_then(|s| s.battle()).map(|g| g.world.tick);
                if let Some(game) = self.session.as_mut().and_then(|s| s.battle_mut()) {
                    tracing::info!("命令攻击 → #{}（选中 {:?}）", target.0, selected);
                    game.order_attack(&selected, target);
                }
                self.clear_order_tool_modes();
                if let Some(tick) = tick {
                    self.pulse_action_lines_at(tick);
                }
            }
            WorldClickIntent::Capture { target } => {
                let selected = self.local.selected.clone();
                let tick = self.session.as_ref().and_then(|s| s.battle()).map(|g| g.world.tick);
                if let Some(game) = self.session.as_mut().and_then(|s| s.battle_mut()) {
                    tracing::info!("命令占领 → #{}（选中 {:?}）", target.0, selected);
                    game.order_capture_building(&selected, target);
                }
                self.clear_order_tool_modes();
                if let Some(tick) = tick {
                    self.pulse_action_lines_at(tick);
                }
            }
            WorldClickIntent::Infiltrate { target } => {
                let selected = self.local.selected.clone();
                let tick = self.session.as_ref().and_then(|s| s.battle()).map(|g| g.world.tick);
                if let Some(game) = self.session.as_mut().and_then(|s| s.battle_mut()) {
                    tracing::info!("命令渗透 → #{}（选中 {:?}）", target.0, selected);
                    game.order_infiltrate(&selected, target);
                }
                self.clear_order_tool_modes();
                if let Some(tick) = tick {
                    self.pulse_action_lines_at(tick);
                }
            }
            WorldClickIntent::Move { cell, queue_path } => {
                let selected = self.local.selected.clone();
                let tick = self.session.as_mut().and_then(|s| s.battle_mut()).map(|game| {
                    tracing::info!("命令移动 → ({},{})（选中 {:?}）", cell.0, cell.1, selected);
                    game.order_move(&selected, cell.0, cell.1);
                    game.world.tick
                });
                if !queue_path {
                    self.planning_waypoints.clear();
                }
                self.clear_order_tool_modes();
                if let Some(tick) = tick {
                    self.pulse_action_lines_at(tick);
                }
            }
            WorldClickIntent::AttackMove { cell } => {
                let selected = self.local.selected.clone();
                let tick = self.session.as_mut().and_then(|s| s.battle_mut()).map(|game| {
                    tracing::info!("命令攻击移动 → ({},{})（选中 {:?}）", cell.0, cell.1, selected);
                    game.order_attack_move(&selected, cell.0, cell.1);
                    game.world.tick
                });
                self.planning_waypoints.clear();
                self.clear_order_tool_modes();
                if let Some(tick) = tick {
                    self.pulse_action_lines_at(tick);
                }
            }
            WorldClickIntent::QueueMovePath { cell } => {
                if self.planning_waypoints.last().copied() != Some(cell) {
                    self.planning_waypoints.push(cell);
                }
                let points = self.planning_waypoints.clone();
                let selected = self.local.selected.clone();
                let tick = self.session.as_mut().and_then(|s| s.battle_mut()).map(|game| {
                    tracing::info!(count = points.len(), "Shift 路径 · 下发 MovePath → ({},{})", cell.0, cell.1);
                    game.order_move_path(&selected, &points);
                    game.world.tick
                });
                if let Some(tick) = tick {
                    self.pulse_action_lines_at(tick);
                }
            }
            WorldClickIntent::SetRally { cell } => {
                let selected = self.local.selected.clone();
                let tick = self.session.as_mut().and_then(|s| s.battle_mut()).map(|game| {
                    tracing::info!("设置集结点 → ({},{})（选中 {:?}）", cell.0, cell.1, selected);
                    game.order_rally(&selected, cell.0, cell.1);
                    game.world.tick
                });
                if let Some(tick) = tick {
                    self.pulse_action_lines_at(tick);
                }
            }
        }
    }

    /// 攻击 / 移动等下令后退出攻击移动与跟随工具态。
    pub(super) fn clear_order_tool_modes(&mut self) {
        if self.interaction_mode.is_attack_move() || self.interaction_mode.is_follow() {
            self.interaction_mode = BattleInteractionMode::Normal;
        }
    }

    /// 框选：按实体屏幕包围盒与拖拽矩形相交，选中本方可控移动单位。
    pub(super) fn handle_marquee_select(&mut self, renderer: &Renderer, window: &Window, rect: ScreenRect) {
        self.sync_present_tick_fraction();
        let add = self.input_tracker.modifiers.shift;
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
        self.ingest_event(event, renderer, window, accept_commands)
    }

    /// 窗口事件归一化：只更新输入态 / 捕获 / 手势，再解析瞬时动作。
    pub(super) fn ingest_event(
        &mut self,
        event: &WindowEvent,
        renderer: &mut Renderer,
        window: &Window,
        accept_commands: bool,
    ) -> BattleNav {
        let battle_paused = self.session.as_ref().and_then(|s| s.battle()).is_some_and(|g| g.paused);
        let script_locked = self.session.as_ref().and_then(|s| s.battle()).is_some_and(|g| g.world.trigger_runtime.script_input_locked);
        let gameplay_open = accept_commands && !battle_paused && !script_locked;
        let outcome_hold =
            self.session.as_ref().and_then(|s| s.battle()).is_some_and(|g| g.outcome.is_some() || g.pending_savour_outcome.is_some());
        // 收束窗 / EVA 播报：仍在 Battle 页，但不再接受对局/暂停输入。
        if accept_commands && outcome_hold {
            match event {
                WindowEvent::CursorMoved { position, .. } => {
                    self.set_cursor_from_physical(window, *position);
                }
                // 非光标事件：清瞬时按住态，避免收束期残留到回局。
                _ => self.reset_transient_input_state(true),
            }
            return BattleNav::None;
        }
        match event {
            WindowEvent::ModifiersChanged(mods) => {
                self.input_tracker.set_modifiers(mods.state().shift_key(), mods.state().control_key(), mods.state().alt_key());
                BattleNav::None
            }
            WindowEvent::MouseInput { state, button: MouseButton::Left, .. } if accept_commands && battle_paused => {
                self.input_tracker.set_left(*state == ElementState::Pressed);
                self.handle_pause_menu_mouse(*state, window)
            }
            WindowEvent::MouseInput { state, button: MouseButton::Left, .. } if gameplay_open => {
                self.input_tracker.set_left(*state == ElementState::Pressed);
                match state {
                    ElementState::Pressed => {
                        self.begin_left_capture(window);
                        BattleNav::None
                    }
                    ElementState::Released => self.end_left_capture(renderer, window),
                }
            }
            WindowEvent::MouseInput { state: ElementState::Released, button: MouseButton::Left, .. } if !accept_commands => {
                self.input_tracker.set_left(false);
                self.reset_transient_input_state(false);
                BattleNav::None
            }
            WindowEvent::MouseInput { state: ElementState::Pressed, button: MouseButton::Right, .. } if gameplay_open => {
                self.input_tracker.set_right(true);
                self.clear_pointer_capture();
                self.handle_right_click(renderer, window);
                BattleNav::None
            }
            WindowEvent::MouseInput { state: ElementState::Released, button: MouseButton::Right, .. } => {
                self.input_tracker.set_right(false);
                BattleNav::None
            }
            WindowEvent::CursorMoved { position, .. } => {
                self.set_cursor_from_physical(window, *position);
                if battle_paused {
                    self.left_gesture = LeftGesture::Idle;
                    self.refresh_pause_hover(window);
                    self.handle_pause_layer_drag(window);
                }
                else if script_locked {
                    self.reset_transient_input_state(false);
                }
                else {
                    let frame = self.input_frame(window);
                    // 仅战术区捕获才推进框选；HUD 按下后移入战术区不得改捕获；放置禁用框选升级。
                    let placing = self.interaction_mode.place_type_id().is_some();
                    if super::super::battle_input::should_advance_world_gesture(accept_commands, placing, frame.capture) {
                        self.left_gesture = self.left_gesture.on_cursor_moved(frame.cursor.0, frame.cursor.1);
                    }
                    self.refresh_command_hover(window);
                }
                BattleNav::None
            }
            WindowEvent::Focused(focused) => {
                self.input_tracker.set_focused(*focused);
                if !*focused {
                    // 失焦保留工具模式，但必须清按住态与修饰键，避免幽灵输入。
                    self.reset_transient_input_state(false);
                }
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
                        self.input_tracker.add_wheel_steps(steps);
                        self.scroll_cameos(window, steps);
                    }
                }
                BattleNav::None
            }
            WindowEvent::KeyboardInput { event, .. } => self.ingest_keyboard(event, renderer, window, accept_commands, gameplay_open, battle_paused, script_locked),
            _ => BattleNav::None,
        }
    }

    /// 清空指针捕获与左键手势（右键取消 / 失焦路径共用）。
    pub(super) fn clear_pointer_capture(&mut self) {
        self.ui_capture = super::super::battle_input::capture_after_right_press();
        self.sidebar_capture_hit = None;
        self.left_gesture = LeftGesture::Idle;
    }

    /// 左键按下：按 down 位置锁定唯一捕获（HUD 与战术区互斥）。
    pub(super) fn begin_left_capture(&mut self, window: &Window) {
        let frame = self.input_frame(window);
        let (x, y) = frame.cursor_i32();
        self.sidebar_capture_hit = None;
        let (command_slot, sidebar_hit) = match self.hit_hud_at(window, x, y) {
            Some(BattleHudHit::CommandButton(slot)) => (Some(slot), None),
            Some(
                hit @ (BattleHudHit::SidebarTab(_)
                | BattleHudHit::Cameo(_)
                | BattleHudHit::Repair
                | BattleHudHit::Sell
                | BattleHudHit::Options
                | BattleHudHit::Diplomacy
                | BattleHudHit::Radar),
            ) => (None, Some(hit)),
            None => (None, None),
        };
        let capture = super::super::battle_input::resolve_press_capture(command_slot, sidebar_hit.is_some(), frame.cursor_in_world);
        self.ui_capture = capture;
        match capture {
            BattleUiCapture::HudCommand(_) => {
                self.left_gesture = LeftGesture::Idle;
            }
            BattleUiCapture::HudSidebar => {
                self.sidebar_capture_hit = sidebar_hit;
                self.left_gesture = LeftGesture::Idle;
            }
            BattleUiCapture::World => {
                // 放置模式仍 begin：禁用框选升级由 `should_advance_world_gesture` 保证，释放仍为 Click。
                self.left_gesture = LeftGesture::begin(frame.cursor.0, frame.cursor.1);
            }
            BattleUiCapture::None | BattleUiCapture::PauseMenu => {
                self.clear_pointer_capture();
            }
        }
    }

    /// 左键释放：只认按下时的捕获；HUD 须同控件抬起，战术区走手势结果。
    pub(super) fn end_left_capture(&mut self, renderer: &mut Renderer, window: &Window) -> BattleNav {
        use super::super::battle_input::LeftReleasePolicy;
        let frame = self.input_frame(window);
        let (x, y) = frame.cursor_i32();
        let capture = self.ui_capture;
        let sidebar_hit = self.sidebar_capture_hit.take();
        self.ui_capture = BattleUiCapture::None;
        match super::super::battle_input::left_release_policy(capture) {
            LeftReleasePolicy::HudCommand(slot) => {
                self.left_gesture = LeftGesture::Idle;
                let release_slot = match self.hit_hud_at(window, x, y) {
                    Some(BattleHudHit::CommandButton(s)) => Some(s),
                    _ => None,
                };
                if super::super::battle_input::hud_command_release_fires(slot, release_slot) {
                    self.on_command_button(slot);
                }
                BattleNav::None
            }
            LeftReleasePolicy::HudSidebar => {
                self.left_gesture = LeftGesture::Idle;
                let Some(hit) = sidebar_hit
                else {
                    return BattleNav::None;
                };
                let same = self.hit_hud_at(window, x, y) == Some(hit);
                if super::super::battle_input::hud_sidebar_release_fires(same) {
                    if hit == BattleHudHit::Radar {
                        self.focus_radar_click(renderer, window, x, y);
                        BattleNav::None
                    }
                    else {
                        self.on_sidebar_hit(hit)
                    }
                }
                else {
                    BattleNav::None
                }
            }
            LeftReleasePolicy::WorldGesture => {
                let (idle, action) = self.left_gesture.release();
                self.left_gesture = idle;
                match action {
                    LeftReleaseAction::None => {}
                    LeftReleaseAction::Click => self.handle_left_click(renderer, window),
                    LeftReleaseAction::Marquee(rect) => self.handle_marquee_select(renderer, window, rect),
                }
                BattleNav::None
            }
            LeftReleasePolicy::Ignore => {
                self.left_gesture = LeftGesture::Idle;
                BattleNav::None
            }
        }
    }

    fn ingest_keyboard(
        &mut self,
        event: &winit::event::KeyEvent,
        renderer: &mut Renderer,
        window: &Window,
        accept_commands: bool,
        gameplay_open: bool,
        battle_paused: bool,
        script_locked: bool,
    ) -> BattleNav {
                let PhysicalKey::Code(code) = event.physical_key
                else {
                    return BattleNav::None;
                };
                let down = event.state == ElementState::Pressed;
                let vk = super::super::battle_hotkeys::key_code_to_vk(code);
                let mods = self.input_tracker.modifiers;
                let hotkey = vk.and_then(|vk| self.hotkeys.action_for(vk, mods.shift, mods.ctrl, mods.alt));

                // 方向键：持续镜头平移与 `keyboard.ini` 瞬时热键解耦。
                // 可玩时始终更新 `camera_pan_keys`；若该键同时被热键表占用，按下仍走热键分发。
                if matches!(code, KeyCode::ArrowLeft | KeyCode::ArrowRight | KeyCode::ArrowUp | KeyCode::ArrowDown) {
                    if !gameplay_open {
                        // 暂停 / 锁输入：静默清空按住，不记抬起边沿（恢复后不应自动续平移）。
                        self.camera_pan_keys.clear();
                        return BattleNav::None;
                    }
                    let prev = self.camera_pan_keys;
                    match code {
                        KeyCode::ArrowLeft => self.camera_pan_keys.left = down,
                        KeyCode::ArrowRight => self.camera_pan_keys.right = down,
                        KeyCode::ArrowUp => self.camera_pan_keys.up = down,
                        KeyCode::ArrowDown => self.camera_pan_keys.down = down,
                        _ => {}
                    }
                    self.input_tracker.note_camera_pan(prev, self.camera_pan_keys);
                    // 未被热键占用，或按键抬起：只更新平移态。
                    if hotkey.is_none() || !down {
                        return BattleNav::None;
                    }
                    // 已被占用且按下：继续落入下方热键分发（平移态已写入）。
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
                let next = if self.interaction_mode.is_repair() { BattleInteractionMode::Normal } else { BattleInteractionMode::Repair };
                if self.interaction_mode.is_planning() {
                    self.planning_waypoints.clear();
                }
                self.interaction_mode = next;
                tracing::info!(active = self.interaction_mode.is_repair(), "ToggleRepair");
                BattleNav::None
            }
            HotkeyAction::ToggleSell => {
                let next = if self.interaction_mode.is_sell() { BattleInteractionMode::Normal } else { BattleInteractionMode::Sell };
                if self.interaction_mode.is_planning() {
                    self.planning_waypoints.clear();
                }
                self.interaction_mode = next;
                tracing::info!(active = self.interaction_mode.is_sell(), "ToggleSell");
                BattleNav::None
            }
            HotkeyAction::PlanningMode => {
                if self.interaction_mode.is_planning() {
                    self.commit_planning_waypoints();
                }
                else {
                    self.planning_waypoints.clear();
                    self.interaction_mode = BattleInteractionMode::Planning;
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
        self.interaction_mode = BattleInteractionMode::Normal;
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
