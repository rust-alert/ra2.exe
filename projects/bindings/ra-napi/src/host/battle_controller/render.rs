//! 对局页控制器：输入意图、命令、tick、快照；不含窗口与页面导航外壳。

use std::{sync::Arc, time::Instant};

use ra_assets::{CsfFile, FntFile, tiberium_overlay_display_hsv_bound};
use ra_engine::{HudSnapshot, terrain_spawner_frame_signature};
use ra_layout::{
    BattleHudChromeMetrics, MapViewport, SIDEBAR_TAB_COUNT, cameo_visible_slot_count, rect_px_from_snapshot, solve_battle_hud_with_metrics,
};
use ra_map::{
    MapEntity, MapEntityKind, OverlayLayerFilter, TILE_HEIGHT, TILE_WIDTH, collect_structure_anim_bank, iso_to_screen,
    paint_ore_tree_frames_onto_rgba, paint_overlays_onto_preview_rgba, paint_structure_anims_onto_rgba, paint_structures_onto_rgba,
    paint_terrain_anims_onto_rgba,
};
use ra_renderer::{Renderer, RgbaImage};
use ra_types::PresentFeel;
use ra_widgets::{
    battle_hud::BattleCameoPaint,
    compose::{BattleHudModel, blit_rgba, compose_battle_hud_overlay, compose_battle_pause_menu_overlay},
    fs_source::GameAssetSource,
    render::present,
    skin::text::{command_button_csf_tooltip, resolve_csf_text},
};
use winit::window::Window;

use super::super::{battle_input::ScreenRect, boot::remap_owner_palette};

use super::BattleController;

impl BattleController {
    /// 绘制当前对局：首帧或空槽全量同步，其后脏集增量。屏上右侧 HUD 由 `HudSnapshot` 驱动。
    pub fn draw_frame(
        &mut self,
        renderer: &mut Renderer,
        window: Option<&Arc<Window>>,
        screen_label: &str,
        fnt: Option<&FntFile>,
        csf: Option<&CsfFile>,
        assets: Option<&GameAssetSource>,
        present: PresentFeel,
    ) {
        enum PendingDraw {
            Full(ra_engine::RenderSnapshot),
            Incremental { tick: u64, dirty: Vec<ra_types::EntityId>, units: Vec<ra_engine::SnapshotUnit> },
        }

        let selected = self.local.selected.clone();
        let force_full = renderer.render_world().unit_count() == 0;
        let prepared = {
            let Some(session) = self.session.as_mut()
            else {
                renderer.draw_frame(None);
                self.refresh_title(renderer, window, screen_label, None);
                return;
            };
            let Some(game) = session.battle_mut()
            else {
                renderer.draw_frame(None);
                self.refresh_title(renderer, window, screen_label, None);
                return;
            };
            let pres_started = Instant::now();
            if force_full {
                let snap = game.snapshot(&selected);
                renderer.timings.presentation_build = Some(pres_started.elapsed());
                let hud = game.snapshot_hud();
                let _ = game.world.take_presentation_dirty();
                (hud, PendingDraw::Full(snap))
            }
            else {
                let dirty = game.world.take_presentation_dirty();
                let units = game.project_units(&dirty);
                let tick = game.world.tick;
                renderer.timings.presentation_build = Some(pres_started.elapsed());
                let hud = game.snapshot_hud();
                (hud, PendingDraw::Incremental { tick, dirty, units })
            }
        };
        let (hud, pending) = prepared;
        self.ensure_battle_hud_chrome(assets);
        self.ensure_pause_menu_chrome(assets);
        self.ensure_order_icons(renderer, assets);
        self.ensure_selection_overlay(renderer, assets);
        self.ensure_cameo_cache(assets);
        self.ensure_start_view(renderer);
        let (vw, vh) = window
            .map(|w| {
                let s = w.inner_size();
                (s.width.max(1), s.height.max(1))
            })
            .unwrap_or((800, 600));
        self.sync_world_view(renderer, vw, vh);
        let hover = self.tactical_hover_entity(renderer, window);
        renderer.set_selection_status(hover, self.condition_yellow, self.condition_red);
        self.tick_deploy_visuals(assets, renderer);
        let overlay_patched = assets.map(|a| self.apply_overlay_paint_dirty(a)).unwrap_or(false);
        let structure_patched = assets.map(|a| self.apply_structure_paint_dirty(a)).unwrap_or(false);
        if self.pending_buildups.is_empty() {
            // 移动单位烤在预览底图上：脏集或仍在滑移时都要重绘（含渲染帧格内插值）。
            let mobiles_moved = match &pending {
                PendingDraw::Incremental { dirty, .. } => self.dirty_includes_mobile(dirty),
                PendingDraw::Full(_) => false,
            };
            let mobiles_sliding = self.any_mobile_sliding();
            if mobiles_moved || mobiles_sliding || overlay_patched || structure_patched {
                if let Some(assets) = assets {
                    self.rebuild_preview_base_with_mobiles(assets);
                    self.present_preview_base(renderer);
                }
            }
            else {
                self.refresh_structure_anims(renderer);
            }
        }
        self.upload_battle_hud(renderer, &hud, fnt, csf, vw, vh, present, screen_label);
        renderer.set_action_lines_active(self.action_lines_active());
        match pending {
            PendingDraw::Full(snap) => renderer.draw_frame(Some(&snap)),
            PendingDraw::Incremental { tick, dirty, units } => renderer.draw_incremental(tick, &dirty, &units, &selected),
        }
        self.refresh_title(renderer, window, screen_label, Some(&hud));
    }

    /// 脏集是否含存活移动单位（步兵 / 载具 / 飞行器）。
    pub(super) fn dirty_includes_mobile(&self, dirty: &[ra_types::EntityId]) -> bool {
        let Some(game) = self.session.as_ref().and_then(|s| s.battle())
        else {
            return false;
        };
        dirty.iter().any(|&id| {
            game.world.ecs_health(id).is_some_and(|(_, _, dead)| !dead)
                && game
                    .world
                    .ecs_identity(id)
                    .is_some_and(|(_, kind)| matches!(kind, MapEntityKind::Unit | MapEntityKind::Infantry | MapEntityKind::Aircraft))
        })
    }

    /// 是否有移动单位仍在寻路 / 滑移（需每渲染帧重烤以插值格内位置）。
    pub(super) fn any_mobile_sliding(&self) -> bool {
        let Some(game) = self.session.as_ref().and_then(|s| s.battle())
        else {
            return false;
        };
        if game.paused || game.outcome.is_some() {
            return false;
        }
        game.world.entity_ids().iter().any(|&id| {
            if game.world.ecs_health(id).map(|(_, _, dead)| dead).unwrap_or(true) {
                return false;
            }
            if !game
                .world
                .ecs_identity(id)
                .is_some_and(|(_, kind)| matches!(kind, MapEntityKind::Unit | MapEntityKind::Infantry | MapEntityKind::Aircraft))
            {
                return false;
            }
            game.world.ecs_path(id).is_some_and(|p| !p.is_empty()) || game.world.ecs_move_destination(id).is_some_and(|(dx, _)| dx.is_some())
        })
    }

    /// 将产矿/采集写入的 overlay 脏格刷回 `preview_clean`。返回是否实际更新。
    ///
    /// 有 underlay 时：`clean = underlay` + 叠全部可采矿（覆盖加矿与扣矿擦除）。
    /// 无 underlay 时回退为仅叠仍存在的脏格。
    pub(super) fn apply_overlay_paint_dirty(&mut self, assets: &GameAssetSource) -> bool {
        let Some(rules) = self.rules.as_ref()
        else {
            return false;
        };
        let overlay_types = rules.overlay_types.clone();
        let color_schemes = rules.color_schemes.clone();
        let dirty = self.session.as_mut().and_then(|s| s.battle_mut()).map(|g| g.world.take_overlay_paint_dirty()).unwrap_or_default();
        if dirty.is_empty() {
            return false;
        }
        let Some(map) = self.session.as_ref().and_then(|s| s.battle()).map(|g| g.world.map.clone())
        else {
            return false;
        };
        let harvestable: Vec<_> = map.overlays.iter().copied().filter(|c| overlay_types.is_harvestable(c.overlay_id)).collect();
        let tib_hsv = |id: u8| {
            let name = overlay_types.name(id)?;
            tiberium_overlay_display_hsv_bound(&color_schemes, name)
        };

        if let Some(underlay) = self.preview_ore_underlay.as_ref() {
            let mut clean = underlay.clone();
            let (shp, mark) = paint_overlays_onto_preview_rgba(
                assets,
                &map,
                &harvestable,
                &mut clean,
                self.preview_origin.0,
                self.preview_origin.1,
                self.art_ini,
                self.rules_ini,
                &|id| overlay_types.name(id).map(str::to_owned),
                &|id| overlay_types.is_harvestable(id),
                &tib_hsv,
                OverlayLayerFilter::Ground,
            );
            let _ = (shp, mark);
            self.preview_clean = Some(clean);
            self.last_anim_sig = u64::MAX;
            return true;
        }

        let cells: Vec<_> = dirty.iter().filter_map(|(x, y)| harvestable.iter().find(|c| c.x == *x && c.y == *y).copied()).collect();
        if cells.is_empty() {
            return false;
        }
        let Some(clean) = self.preview_clean.as_mut()
        else {
            return false;
        };
        let (shp, mark) = paint_overlays_onto_preview_rgba(
            assets,
            &map,
            &cells,
            clean,
            self.preview_origin.0,
            self.preview_origin.1,
            self.art_ini,
            self.rules_ini,
            &|id| overlay_types.name(id).map(str::to_owned),
            &|id| overlay_types.is_harvestable(id),
            &tib_hsv,
            OverlayLayerFilter::Ground,
        );
        if shp + mark == 0 {
            return false;
        }
        self.last_anim_sig = u64::MAX;
        true
    }

    /// 将占领等房主变更的建筑按新房主色烤进 `preview_clean` / underlay，并刷新活动层。
    pub(super) fn apply_structure_paint_dirty(&mut self, assets: &GameAssetSource) -> bool {
        let dirty = self.session.as_mut().and_then(|s| s.battle_mut()).map(|g| g.world.take_structure_paint_dirty()).unwrap_or_default();
        if dirty.is_empty() {
            return false;
        }
        let Some(rules) = self.rules.as_ref()
        else {
            return false;
        };
        let jobs: Vec<(String, String, u16, u16)> = {
            let Some(game) = self.session.as_ref().and_then(|s| s.battle())
            else {
                return false;
            };
            dirty
                .iter()
                .filter_map(|&id| {
                    if game.world.ecs_health(id).map(|(_, _, dead)| dead).unwrap_or(true) {
                        return None;
                    }
                    let (type_id, kind) = game.world.ecs_identity(id)?;
                    if kind != MapEntityKind::Structure {
                        return None;
                    }
                    let owner = game.world.ecs_owner(id)?;
                    let (x, y, _) = game.world.ecs_transform(id)?;
                    Some((type_id.to_string(), owner.to_string(), x, y))
                })
                .collect()
        };
        if jobs.is_empty() {
            return false;
        }
        let art_ini = self.art_ini;
        let origin = self.preview_origin;
        let lobby = self.lobby_primaries.clone();
        let mut any = false;
        for (type_id, owner, x, y) in &jobs {
            let Some(game) = self.session.as_ref().and_then(|s| s.battle())
            else {
                break;
            };
            let mut one = game.world.map.clone();
            one.entities.clear();
            one.entities.push(MapEntity {
                kind: MapEntityKind::Structure,
                owner: owner.clone(),
                type_id: type_id.clone(),
                health: 256,
                x: *x,
                y: *y,
                facing: 0,
                sub_cell: 0,
                mission: String::new(),
                tag: String::new(),
            });
            let remap = |base: &ra_assets::Palette, own: &str| remap_owner_palette(rules, Some(&lobby), base, own);
            if let Some(clean) = self.preview_clean.as_mut() {
                let n = paint_structures_onto_rgba(assets, &one, clean, origin.0, origin.1, art_ini, self.rules_ini, &remap);
                any |= n > 0;
            }
            if let Some(underlay) = self.preview_ore_underlay.as_mut() {
                let n = paint_structures_onto_rgba(assets, &one, underlay, origin.0, origin.1, art_ini, self.rules_ini, &remap);
                any |= n > 0;
            }
            self.structure_anims.layers.retain(|layer| !(layer.x == *x && layer.y == *y));
            let bank = collect_structure_anim_bank(assets, &one, art_ini, self.rules_ini, &remap);
            self.structure_anims.layers.extend(bank.layers);
        }
        if any {
            self.last_anim_sig = u64::MAX;
        }
        any
    }

    /// 按呈现时钟刷新建筑 ActiveAnim（旗帜 / 泵机）、常循环地形、矿柱状态机帧与天气粒子，不重置相机。
    pub(super) fn refresh_structure_anims(&mut self, renderer: &mut Renderer) {
        let Some(base) = self.preview_base.as_ref()
        else {
            return;
        };
        let clock_ms = self.anim_started.elapsed().as_millis() as u64;
        let sig = self.preview_anim_signature(clock_ms);
        let weather_active = self.weather.is_active();
        let has_anims = self.has_preview_anims();
        if !weather_active && (!has_anims || sig == self.last_anim_sig) {
            return;
        }
        let mut composed = base.clone();
        if has_anims {
            paint_terrain_anims_onto_rgba(&mut composed, self.preview_origin.0, self.preview_origin.1, &self.terrain_anims, clock_ms);
            self.paint_ore_tree_frames_onto(&mut composed);
            paint_structure_anims_onto_rgba(&mut composed, self.preview_origin.0, self.preview_origin.1, &self.structure_anims, clock_ms);
        }
        self.paint_weather_onto(&mut composed);
        renderer.update_map_preview(composed);
        self.last_anim_sig = sig;
    }

    /// 是否有需叠画的活动层（建筑 / 常循环地形 / 矿柱）。
    pub(super) fn has_preview_anims(&self) -> bool {
        !self.structure_anims.is_empty() || !self.terrain_anims.is_empty() || !self.ore_tree_anims.is_empty()
    }

    /// 从世界矿柱状态机读取当前帧并叠画。
    pub(super) fn paint_ore_tree_frames_onto(&self, image: &mut RgbaImage) {
        if self.ore_tree_anims.is_empty() {
            return;
        }
        let frames = self.ore_tree_render_frames();
        paint_ore_tree_frames_onto_rgba(image, self.preview_origin.0, self.preview_origin.1, &self.ore_tree_anims, &frames);
    }

    /// `(格x, 格y, 帧)`；无会话时回退 Idle 帧 0。
    pub(super) fn ore_tree_render_frames(&self) -> Vec<(u16, u16, u16)> {
        if let Some(game) = self.session.as_ref().and_then(|s| s.battle()) {
            if !game.world.terrain_spawners.is_empty() {
                return game.world.terrain_spawners.iter().map(|s| (s.x, s.y, s.render_frame())).collect();
            }
        }
        self.ore_tree_anims.layers.iter().map(|layer| (layer.x, layer.y, 0)).collect()
    }

    /// 建筑 + 常循环地形 + 矿柱状态机帧签名（用于跳过无变化上传）。
    pub(super) fn preview_anim_signature(&self, clock_ms: u64) -> u64 {
        let mut h = self.structure_anims.frame_signature(clock_ms);
        h ^= self.terrain_anims.frame_signature(clock_ms).rotate_left(17);
        let spawners = self.session.as_ref().and_then(|s| s.battle()).map(|g| g.world.terrain_spawners.as_slice()).unwrap_or(&[]);
        h ^= terrain_spawner_frame_signature(spawners).rotate_left(29);
        h
    }

    /// 按预览尺寸推进并叠画天气粒子。
    pub(super) fn paint_weather_onto(&mut self, image: &mut RgbaImage) {
        if self.map_theater.is_none() && !self.weather.is_active() {
            return;
        }
        self.weather.resize(image.width(), image.height());
        let now_ms = self.weather_started.elapsed().as_millis() as u64;
        let dt = now_ms.saturating_sub(self.weather_last_ms);
        self.weather_last_ms = now_ms;
        self.weather.tick(dt);
        self.weather.paint_onto(image);
    }

    /// 放置模式下在光标格画占地幽灵：可放绿、不可放红（按格着色）。
    pub(super) fn paint_placement_ghost(&self, page: &mut RgbaImage, renderer: &Renderer, window_w: u32, window_h: u32, type_id: &str) {
        let Some(game) = self.session.as_ref().and_then(|s| s.battle())
        else {
            return;
        };
        let vp = MapViewport::battle(window_w.max(1), window_h.max(1));
        if !vp.contains_cursor(self.cursor.0 as i32, self.cursor.1 as i32) {
            return;
        }
        let (wx, wy) = vp.screen_to_world(renderer.camera(), self.cursor.0 as f32, self.cursor.1 as f32);
        let Some((ox, oy)) = game.image_to_cell(wx, wy)
        else {
            return;
        };
        let foundation = game.world.definitions.structures.get(type_id).map(|s| s.foundation.clone()).unwrap_or_default();
        let width = foundation.width.max(1);
        let height = foundation.height.max(1);
        let cam = renderer.camera();
        let half_w = (TILE_WIDTH / 2) as f32;
        let half_h = (TILE_HEIGHT / 2) as f32;
        for dy in 0..height {
            for dx in 0..width {
                let Some(cx) = ox.checked_add(dx)
                else {
                    continue;
                };
                let Some(cy) = oy.checked_add(dy)
                else {
                    continue;
                };
                let ok = game.world.can_place_structure(cx, cy);
                let fill = if ok { [40u8, 220, 70, 90] } else { [220u8, 40, 40, 110] };
                let stroke = if ok { [80u8, 255, 100, 230] } else { [255u8, 70, 70, 240] };
                let z = game.world.pass_grid.cell_height(cx, cy);
                let (sx, sy) = iso_to_screen(i32::from(cx), i32::from(cy), z);
                let center_wx = (sx - game.preview_origin_x) as f32 + half_w;
                let center_wy = (sy - game.preview_origin_y) as f32 + half_h;
                let corners_w = [
                    (center_wx, center_wy - half_h),
                    (center_wx + half_w, center_wy),
                    (center_wx, center_wy + half_h),
                    (center_wx - half_w, center_wy),
                ];
                let mut corners_s = [(0i32, 0i32); 4];
                for (i, (wx, wy)) in corners_w.iter().copied().enumerate() {
                    let (sx, sy) = vp.world_to_screen(cam, wx, wy);
                    corners_s[i] = (sx.round() as i32, sy.round() as i32);
                }
                fill_screen_diamond(page, &vp, corners_s, fill);
                stroke_screen_diamond(page, &vp, corners_s, stroke);
            }
        }
    }

    pub(super) fn upload_battle_hud(
        &mut self,
        renderer: &mut Renderer,
        hud: &HudSnapshot,
        fnt: Option<&FntFile>,
        csf: Option<&CsfFile>,
        viewport_w: u32,
        viewport_h: u32,
        present: PresentFeel,
        _screen_label: &str,
    ) {
        let w = viewport_w.max(1);
        let h = viewport_h.max(1);
        let local_house = self.local_house_name();
        let local = local_house.as_ref().and_then(|house| hud.players.iter().find(|p| p.house.as_ref() == house.as_str()));
        let nsel = self.local.selected.len();
        let game = self.session.as_ref().and_then(|s| s.battle());
        let selected_type =
            self.local.selected.first().copied().and_then(|id| game.and_then(|g| g.world.ecs_identity(id).map(|(t, _)| t.to_string())));
        let selected_summary = match (self.local.selected.first().copied(), nsel, selected_type.as_deref()) {
            (Some(id), n, Some(ty)) if n > 1 => format!("#{}+{} {ty}", id.0, n - 1),
            (Some(id), _, Some(ty)) => format!("#{} {ty}", id.0),
            (Some(id), n, None) if n > 1 => format!("#{}+{}", id.0, n - 1),
            (Some(id), _, None) => format!("#{}", id.0),
            _ => "—".into(),
        };
        let deploy_hint_owned =
            self.local.selected.first().copied().and_then(|id| game.and_then(|g| g.deploy_target_of(id).map(|t| format!("D→{t}"))));
        let queue = hud.produce_queues.first().map(|q| format!("队列 {}:{}", q.type_id, q.remaining_ticks));
        let reject = hud.last_rejects.first().map(|r| r.reason.as_hud_label());
        let tip_owned = self.command_hover.and_then(command_button_csf_tooltip).and_then(|key| resolve_csf_text(csf, key));
        // 暂停菜单打开时不再画「已暂停」横幅文案。
        let show_pause_banner = hud.paused && hud.outcome.is_none();

        let caps = game.map(|g| g.snapshot_capabilities(&self.local.selected));
        let tabs_visible = Self::sidebar_tabs_visible(caps.as_ref());
        self.sync_sidebar_tab_to_visible(tabs_visible);
        let metrics = self.hud_chrome.as_ref().map(|c| BattleHudChromeMetrics::for_mix(&c.mix)).unwrap_or_else(BattleHudChromeMetrics::sidec01);
        let snap = solve_battle_hud_with_metrics(w, h, metrics);
        let band = rect_px_from_snapshot(&snap, "cameo_band");
        let visible = cameo_visible_slot_count(band.h);
        self.clamp_cameo_scroll(visible);
        let items = caps.as_ref().map(|c| Self::tab_items(c, self.sidebar_tab)).unwrap_or_default();
        let start = self.cameo_scroll.min(items.len());
        let end = (start + visible).min(items.len());
        let page_items = &items[start..end];
        let cameos: Vec<BattleCameoPaint<'_>> = page_items
            .iter()
            .map(|item| {
                let key = item.type_id.as_ref();
                let progress = hud
                    .produce_queues
                    .iter()
                    .filter(|q| q.type_id.as_ref().eq_ignore_ascii_case(key))
                    .map(|q| {
                        if q.remaining_ticks == 0 {
                            1.0
                        }
                        else if q.total_ticks == 0 {
                            0.0
                        }
                        else {
                            1.0 - (q.remaining_ticks as f32 / q.total_ticks as f32)
                        }
                    })
                    .fold(None, |best: Option<f32>, p| Some(best.map_or(p, |b| b.max(p))));
                BattleCameoPaint {
                    type_id: key,
                    image: self.cameo_cache.get(key).and_then(|opt| opt.as_ref()).map(|s| &s.image),
                    enabled: item.enabled,
                    selected: matches!(self.sidebar_tab, 0 | 1)
                        && (self.place_mode.as_deref() == Some(key)
                            || self.session.as_ref().and_then(|s| s.battle()).is_some_and(|g| g.is_local_ready_to_place(key))),
                    progress,
                }
            })
            .collect();

        let paint = BattleHudModel {
            tick: hud.tick,
            funds: local.map(|p| p.funds).unwrap_or(0),
            power_output: local.map(|p| p.power_output).unwrap_or(0),
            power_drain: local.map(|p| p.power_drain).unwrap_or(0),
            low_power: local.map(|p| p.low_power).unwrap_or(false),
            selected_summary: selected_summary.as_str(),
            deploy_hint: deploy_hint_owned.as_deref(),
            produce_queue: queue.as_deref(),
            reject,
            paused: show_pause_banner,
            pause_reason: None,
            command_pressed: if show_pause_banner {
                None
            }
            else {
                self.command_pressed.or_else(|| {
                    if self.planning_mode {
                        ra_widgets::skin::text::SKIRMISH_COMMAND_BAR.iter().position(|&n| n == "PlanningMode")
                    }
                    else {
                        None
                    }
                })
            },
            command_hovered: if show_pause_banner { None } else { self.command_hover },
            command_tip: if show_pause_banner { None } else { tip_owned.as_deref() },
            repair_active: !show_pause_banner && self.repair_mode,
            sell_active: !show_pause_banner && self.sell_mode,
            radar_online: !show_pause_banner && !local.map(|p| p.low_power).unwrap_or(false) && caps.as_ref().is_some_and(|c| c.has_radar),
            sidebar_tab: self.sidebar_tab.min(SIDEBAR_TAB_COUNT.saturating_sub(1)),
            sidebar_tabs_visible: tabs_visible,
            cameos: if show_pause_banner { &[] } else { &cameos },
        };
        // 与命中 / `world_viewport` 同口径：按窗口像素合成，避免 800×600 letterbox 错位。
        if let Some(mut page) = compose_battle_hud_overlay(w, h, fnt, paint, self.hud_chrome.as_ref()) {
            if let Some(rect) = self.left_gesture.marquee_rect() {
                stroke_marquee_rect(&mut page, rect);
            }
            if !show_pause_banner {
                if let Some(type_id) = self.place_mode.clone() {
                    self.paint_placement_ghost(&mut page, renderer, w, h, &type_id);
                }
            }
            if show_pause_banner {
                let metrics = self.pause_hud_metrics();
                if let Some(pause) = compose_battle_pause_menu_overlay(
                    w,
                    h,
                    self.pause_pressed,
                    self.pause_hover,
                    fnt,
                    csf,
                    self.pause_menu_chrome.as_ref(),
                    metrics,
                ) {
                    blit_rgba(&mut page, &pause, 0, 0);
                }
            }
            // 与壳层菜单同走 `[present]`，避免对局侧栏仍以满 8-bit 显得过亮。
            let page = present::present_ui_page(page, present);
            renderer.set_ui_overlay(page);
        }
    }

    pub(super) fn refresh_title(&mut self, renderer: &Renderer, window: Option<&Arc<Window>>, screen_label: &str, hud: Option<&HudSnapshot>) {
        if let Some(window) = window {
            let zoom = renderer.camera().zoom;
            let title = if let Some(hud) = hud {
                let local_house = self
                    .session
                    .as_ref()
                    .and_then(|s| s.battle())
                    .and_then(|g| g.world.players.iter().find(|p| p.id == g.world.local_player))
                    .map(|p| p.house.clone());
                let local = local_house.and_then(|house| hud.players.iter().find(|p| p.house == house));
                let econ = local
                    .map(|p| {
                        let low = if p.low_power { "!" } else { "" };
                        format!("${} 电{}/{}{low}", p.funds, p.power_output, p.power_drain)
                    })
                    .unwrap_or_else(|| "$-".into());
                let queue =
                    hud.produce_queues.first().map(|q| format!("q:{}:{}", q.type_id, q.remaining_ticks)).unwrap_or_else(|| "q:-".into());
                let reject = hud.last_rejects.first().map(|r| r.reason.as_hud_label()).unwrap_or("-");
                let place = self.place_mode.as_deref().unwrap_or("-");
                if screen_label == "results" {
                    format!("{} · [results] · t{} · Enter确认 Esc离开", self.title_base, hud.tick)
                }
                else if hud.paused {
                    format!("{} · [{screen_label}] · t{} · 暂停菜单 · Esc/回到游戏 · 放弃回大厅", self.title_base, hud.tick)
                }
                else if self.place_mode.is_some() {
                    let nsel = self.local.selected.len();
                    let sel = self.local.selected.first().copied();
                    let sel_part = match (sel, nsel) {
                        (Some(id), n) if n > 1 => format!("#{}+{}", id.0, n - 1),
                        (Some(id), _) => format!("#{}", id.0),
                        (None, _) => "#-".into(),
                    };
                    let diff = self.session.as_ref().and_then(|s| s.battle()).map(|g| g.difficulty.as_str()).unwrap_or("Normal");
                    format!(
                        "{} · [{screen_label}] · t{} · {econ} · {queue} · 建:{place} · {reject} · {sel_part} · diff={diff} · Esc取消建造 · z{:.2}",
                        self.title_base, hud.tick, zoom
                    )
                }
                else {
                    let nsel = self.local.selected.len();
                    let sel = self.local.selected.first().copied();
                    let sel_part = match (sel, nsel) {
                        (Some(id), n) if n > 1 => format!("#{}+{}", id.0, n - 1),
                        (Some(id), _) => format!("#{}", id.0),
                        (None, _) => "#-".into(),
                    };
                    let diff = self.session.as_ref().and_then(|s| s.battle()).map(|g| g.difficulty.as_str()).unwrap_or("Normal");
                    format!(
                        "{} · [{screen_label}] · t{} · {econ} · {queue} · 建:{place} · {reject} · {sel_part} · diff={diff} · Esc暂停 · z{:.2}",
                        self.title_base, hud.tick, zoom
                    )
                }
            }
            else {
                format!("{} · [{screen_label}] · z{:.2}", self.title_base, zoom)
            };
            window.set_title(&title);
        }
        if let Some(game) = self.session.as_ref().and_then(|s| s.battle()) {
            if let Some(reject) = game.world.last_rejects().first() {
                let label = reject.reason.as_hud_label().to_string();
                if self.logged_reject.as_deref() != Some(label.as_str()) {
                    self.logged_reject = Some(label.clone());
                    tracing::info!("命令拒绝 · {label}");
                }
            }
        }
        if let (Some(path), Some(session)) = (self.status_path.as_ref(), self.session.as_ref()) {
            #[cfg(feature = "test-harness")]
            super::super::test_boot::write_status(path, session, &self.local.selected, screen_label, self.leave_armed);
            #[cfg(not(feature = "test-harness"))]
            let _ = (path, session);
        }
    }
}

/// 在 HUD 叠加层上描框选矩形（半透明黄绿边）。
fn stroke_marquee_rect(page: &mut RgbaImage, rect: ScreenRect) {
    let w = page.width() as i32;
    let h = page.height() as i32;
    if w <= 0 || h <= 0 || rect.w < 1.0 || rect.h < 1.0 {
        return;
    }
    let x0 = rect.x.floor() as i32;
    let y0 = rect.y.floor() as i32;
    let x1 = (rect.x + rect.w).ceil() as i32;
    let y1 = (rect.y + rect.h).ceil() as i32;
    let color = [180u8, 255, 60, 220];
    let put = |img: &mut RgbaImage, x: i32, y: i32| {
        if x < 0 || y < 0 || x >= w || y >= h {
            return;
        }
        let i = ((y as u32 * img.width() + x as u32) * 4) as usize;
        let px = img.as_mut();
        px[i] = color[0];
        px[i + 1] = color[1];
        px[i + 2] = color[2];
        px[i + 3] = color[3];
    };
    for x in x0..=x1 {
        put(page, x, y0);
        put(page, x, y1);
        if y0 + 1 < y1 {
            put(page, x, y0 + 1);
            put(page, x, y1 - 1);
        }
    }
    for y in y0..=y1 {
        put(page, x0, y);
        put(page, x1, y);
        if x0 + 1 < x1 {
            put(page, x0 + 1, y);
            put(page, x1 - 1, y);
        }
    }
}

/// 在战术区内半透明填充屏幕空间菱形（四顶点，顺时针或任意凸四边形近似）。
fn fill_screen_diamond(page: &mut RgbaImage, vp: &MapViewport, corners: [(i32, i32); 4], color: [u8; 4]) {
    let ymin = corners.iter().map(|c| c.1).min().unwrap_or(0);
    let ymax = corners.iter().map(|c| c.1).max().unwrap_or(0);
    if ymax < ymin {
        return;
    }
    for y in ymin..=ymax {
        let mut xs = [i32::MAX, i32::MIN];
        for i in 0..4 {
            let (x0, y0) = corners[i];
            let (x1, y1) = corners[(i + 1) % 4];
            if (y0 <= y && y1 > y) || (y1 <= y && y0 > y) {
                let t = (y - y0) as f32 / (y1 - y0) as f32;
                let x = x0 as f32 + t * (x1 - x0) as f32;
                let xi = x.round() as i32;
                xs[0] = xs[0].min(xi);
                xs[1] = xs[1].max(xi);
            }
            else if y0 == y && y1 == y {
                xs[0] = xs[0].min(x0.min(x1));
                xs[1] = xs[1].max(x0.max(x1));
            }
        }
        if xs[0] == i32::MAX || xs[1] == i32::MIN {
            continue;
        }
        for x in xs[0]..=xs[1] {
            if vp.contains_cursor(x, y) {
                blend_overlay_pixel(page, x, y, color);
            }
        }
    }
}

/// 描菱形边框。
fn stroke_screen_diamond(page: &mut RgbaImage, vp: &MapViewport, corners: [(i32, i32); 4], color: [u8; 4]) {
    for i in 0..4 {
        let (x0, y0) = corners[i];
        let (x1, y1) = corners[(i + 1) % 4];
        stroke_screen_line(page, vp, x0, y0, x1, y1, color);
    }
}

pub(super) fn stroke_screen_line(page: &mut RgbaImage, vp: &MapViewport, x0: i32, y0: i32, x1: i32, y1: i32, color: [u8; 4]) {
    let dx = (x1 - x0).abs();
    let dy = (y1 - y0).abs();
    let sx = if x0 < x1 { 1 } else { -1 };
    let sy = if y0 < y1 { 1 } else { -1 };
    let mut err = dx - dy;
    let mut x = x0;
    let mut y = y0;
    loop {
        if vp.contains_cursor(x, y) {
            blend_overlay_pixel(page, x, y, color);
        }
        if x == x1 && y == y1 {
            break;
        }
        let e2 = err * 2;
        if e2 > -dy {
            err -= dy;
            x += sx;
        }
        if e2 < dx {
            err += dx;
            y += sy;
        }
    }
}

pub(super) fn blend_overlay_pixel(page: &mut RgbaImage, x: i32, y: i32, rgba: [u8; 4]) {
    let w = page.width() as i32;
    let h = page.height() as i32;
    if x < 0 || y < 0 || x >= w || y >= h {
        return;
    }
    let i = ((y as u32 * page.width() + x as u32) * 4) as usize;
    let px = page.as_mut();
    let src_a = u32::from(rgba[3]);
    if src_a == 0 {
        return;
    }
    if src_a >= 255 {
        px[i] = rgba[0];
        px[i + 1] = rgba[1];
        px[i + 2] = rgba[2];
        px[i + 3] = 255;
        return;
    }
    let dst_a = u32::from(px[i + 3]);
    let out_a = src_a + dst_a * (255 - src_a) / 255;
    if out_a == 0 {
        px[i] = 0;
        px[i + 1] = 0;
        px[i + 2] = 0;
        px[i + 3] = 0;
        return;
    }
    let blend = |s: u8, d: u8| -> u8 {
        let s = u32::from(s);
        let d = u32::from(d);
        ((s * src_a + d * dst_a * (255 - src_a) / 255) / out_a) as u8
    };
    px[i] = blend(rgba[0], px[i]);
    px[i + 1] = blend(rgba[1], px[i + 1]);
    px[i + 2] = blend(rgba[2], px[i + 2]);
    px[i + 3] = out_a as u8;
}
