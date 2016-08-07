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

use super::{BATTLE_START_ZOOM, BattleController, VIEW_BOOKMARK_COUNT, ViewBookmark};


impl BattleController {
    /// 表面尺寸就绪后对齐战术区并聚焦开局单位（可重复调用，只执行一次）。
    pub fn ensure_start_view(&mut self, renderer: &mut Renderer) {
        if !self.start_view_pending {
            return;
        }
        let Some((vw, vh)) = renderer.surface_size_u32()
        else {
            return;
        };
        self.sync_world_view(renderer, vw, vh);
        self.sync_camera_content_bounds(renderer);
        self.focus_camera_on_local_start(renderer);
        self.start_view_pending = false;
    }

    /// 按 `[Map] LocalSize` 投影设置镜头内容矩形，避免扫到 `Size` 外缘锯齿外。
    pub(super) fn sync_camera_content_bounds(&self, renderer: &mut Renderer) {
        let Some(game) = self.session.as_ref().and_then(|s| s.battle())
        else {
            renderer.clear_camera_content_rect();
            return;
        };
        let map = &game.world.map;
        let Some((x0, y0, x1, y1)) = local_size_preview_rect(map.size_width, map.local_size, game.preview_origin_x, game.preview_origin_y)
        else {
            renderer.clear_camera_content_rect();
            return;
        };
        let (pw, ph) = match renderer.preview_size_u32() {
            Some(s) => s,
            None => {
                renderer.clear_camera_content_rect();
                return;
            }
        };
        // 与预览画布求交，避免越界内容矩形。
        let x0 = x0.max(0).min(pw.saturating_sub(1) as i32);
        let y0 = y0.max(0).min(ph.saturating_sub(1) as i32);
        let x1 = x1.max(x0 + 1).min(pw as i32);
        let y1 = y1.max(y0 + 1).min(ph as i32);
        renderer.set_camera_content_rect(x0 as f32, y0 as f32, x1 as f32, y1 as f32);
    }

    /// 将镜头对准本地玩家开局单位（遭遇战优先 MCV 出生点附近）。
    pub fn focus_camera_on_local_start(&self, renderer: &mut Renderer) {
        let Some(game) = self.session.as_ref().and_then(|s| s.battle())
        else {
            return;
        };
        let Some(local_house) = game.world.players.iter().find(|p| p.id == game.world.local_player).map(|p| p.house.clone())
        else {
            return;
        };
        let mut fallback: Option<(u16, u16)> = None;
        let mut mcv: Option<(u16, u16)> = None;
        for id in game.world.entity_ids() {
            let Some(owner) = game.world.ecs_owner(id)
            else {
                continue;
            };
            if owner.as_ref() != local_house.as_ref() {
                continue;
            }
            let Some((_, _, dead)) = game.world.ecs_health(id)
            else {
                continue;
            };
            if dead {
                continue;
            }
            let Some((type_id, kind)) = game.world.ecs_identity(id)
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
            if type_id.to_ascii_uppercase().contains("MCV") {
                mcv = Some((x, y));
                break;
            }
            if fallback.is_none() {
                fallback = Some((x, y));
            }
        }
        let Some((x, y)) = mcv.or(fallback)
        else {
            tracing::warn!("本地阵营 {} 无可用开局单位，镜头保持预览 fit", local_house);
            return;
        };
        let z = game.world.pass_grid.cell_height(x, y);
        let (sx, sy) = iso_to_screen(i32::from(x), i32::from(y), z);
        let wx = (sx - game.preview_origin_x) as f32;
        let wy = (sy - game.preview_origin_y) as f32;
        renderer.focus_camera(wx, wy, BATTLE_START_ZOOM);
        tracing::info!("开局镜头对准 {} @({},{}) zoom={}", local_house, x, y, BATTLE_START_ZOOM);
    }

    /// 镜头对准地图格（`CenterView` / `TeamCenter` / 选中居中）。
    pub(super) fn focus_camera_on_cell(&self, renderer: &mut Renderer, x: u16, y: u16) {
        let Some(game) = self.session.as_ref().and_then(|s| s.battle())
        else {
            return;
        };
        let z = game.world.pass_grid.cell_height(x, y);
        let (sx, sy) = iso_to_screen(i32::from(x), i32::from(y), z);
        let wx = (sx - game.preview_origin_x) as f32;
        let wy = (sy - game.preview_origin_y) as f32;
        let zoom = renderer.camera().zoom;
        renderer.focus_camera(wx, wy, zoom);
    }

    /// `SetView`：把当前镜头写入书签槽（`1..=4`）。
    pub(super) fn set_view_bookmark(&mut self, renderer: &Renderer, slot_1_to_4: u8) {
        let Some(idx) = (1..=VIEW_BOOKMARK_COUNT as u8).contains(&slot_1_to_4).then_some(usize::from(slot_1_to_4 - 1))
        else {
            return;
        };
        let cam = renderer.camera();
        self.view_bookmarks[idx] = Some(ViewBookmark {
            center_x: cam.center_x,
            center_y: cam.center_y,
            zoom: cam.zoom,
        });
        tracing::info!(slot = slot_1_to_4, x = cam.center_x, y = cam.center_y, zoom = cam.zoom, "SetView");
    }

    /// `View`：召回书签槽镜头；空槽忽略。
    pub(super) fn recall_view_bookmark(&self, renderer: &mut Renderer, slot_1_to_4: u8) {
        let Some(idx) = (1..=VIEW_BOOKMARK_COUNT as u8).contains(&slot_1_to_4).then_some(usize::from(slot_1_to_4 - 1))
        else {
            return;
        };
        let Some(bm) = self.view_bookmarks[idx]
        else {
            tracing::debug!(slot = slot_1_to_4, "View · 书签为空");
            return;
        };
        renderer.focus_camera(bm.center_x, bm.center_y, bm.zoom);
        tracing::info!(slot = slot_1_to_4, x = bm.center_x, y = bm.center_y, zoom = bm.zoom, "View");
    }

    /// 由窗口尺寸构造当前对局 `MapViewport`（命中 / 投影 / 裁切同一实例）。
    pub(super) fn map_viewport(&self, window: &Window) -> MapViewport {
        let size = window.inner_size();
        MapViewport::battle(size.width.max(1), size.height.max(1))
    }

    /// 将 renderer 世界 pass 与 `MapViewport` 对齐。
    pub(super) fn sync_world_view(&self, renderer: &mut Renderer, window_w: u32, window_h: u32) {
        let vp = MapViewport::battle(window_w.max(1), window_h.max(1));
        let (x, y, w, h) = vp.clip_rect_u32();
        renderer.set_world_view_rect(x, y, w, h);
    }

    pub(super) fn cursor_cell(&self, renderer: &Renderer, window: &Window) -> Option<(u16, u16)> {
        let game = self.session.as_ref()?.battle()?;
        let vp = self.map_viewport(window);
        if !vp.contains_cursor(self.cursor.0 as i32, self.cursor.1 as i32) {
            return None;
        }
        let (wx, wy) = vp.screen_to_world(renderer.camera(), self.cursor.0 as f32, self.cursor.1 as f32);
        game.image_to_cell(wx, wy)
    }

    pub(super) fn pan_world(&self, renderer: &mut Renderer, window: &Window, dx: f32, dy: f32) {
        let vp = self.map_viewport(window);
        // 夹紧与投影同口径：战术区宽高（与 `set_world_view_rect` / write_vertices 一致）。
        renderer.pan_clamped_in_viewport(dx, dy, vp.proj_w(), vp.proj_h());
    }

    /// 镜头平移：整窗边缘滚屏 + 方向键按住连续平移（均按真实 `dt`，与逻辑 tick 无关）。
    pub fn tick_edge_scroll(&mut self, renderer: &mut Renderer, window: &Window, dt: f64, enabled: bool) {
        if !enabled || dt <= 0.0 {
            self.edge_scroll_cursor = EdgeScrollCursor::Default;
            return;
        }
        if self.session.as_ref().and_then(|s| s.battle()).is_some_and(|g| g.paused || g.outcome.is_some()) {
            self.edge_scroll_cursor = EdgeScrollCursor::Default;
            self.camera_pan_keys.clear();
            return;
        }
        let size = window.inner_size();
        let sw = size.width.max(1);
        let sh = size.height.max(1);
        let (west, east, north, south) = edge_scroll_axes(self.cursor.0, self.cursor.1, sw, sh, EDGE_SCROLL_MARGIN_PX);
        let vp = self.map_viewport(window);
        let (can_west, can_east, can_north, can_south) = self.edge_scroll_can_axes(renderer, vp.proj_w(), vp.proj_h());
        self.edge_scroll_cursor = edge_scroll_cursor_for(west, east, north, south, can_west, can_east, can_north, can_south);
        let (mut dx, mut dy) =
            edge_scroll_screen_delta(self.cursor.0, self.cursor.1, sw, sh, EDGE_SCROLL_MARGIN_PX, EDGE_SCROLL_SPEED_PX_PER_SEC, dt);
        let (kx, ky) = keyboard_pan_screen_delta(self.camera_pan_keys, KEYBOARD_PAN_SPEED_PX_PER_SEC, dt);
        dx += kx;
        dy += ky;
        if dx > 0.0 && !can_west {
            dx = 0.0;
        }
        if dx < 0.0 && !can_east {
            dx = 0.0;
        }
        if dy > 0.0 && !can_north {
            dy = 0.0;
        }
        if dy < 0.0 && !can_south {
            dy = 0.0;
        }
        if dx.abs() > 0.0 || dy.abs() > 0.0 {
            self.pan_world(renderer, window, dx, dy);
        }
    }

    /// 当前边缘滚屏光标（壳层据此切换系统 / 自定义指针）。
    pub fn edge_scroll_cursor(&self) -> EdgeScrollCursor {
        self.edge_scroll_cursor
    }

    /// 各轴是否还能平移（`pan_screen`：正 dx 减 `center_x`，正 dy 减 `center_y`）。
    pub(super) fn edge_scroll_can_axes(&self, renderer: &Renderer, proj_w: f32, proj_h: f32) -> (bool, bool, bool, bool) {
        let Some(bounds) = renderer.camera_bounds_for_viewport(proj_w, proj_h)
        else {
            return (true, true, true, true);
        };
        let cam = renderer.camera();
        const EPS: f32 = 0.5;
        let can_west = cam.center_x > bounds.min_center_x + EPS;
        let can_east = cam.center_x < bounds.max_center_x - EPS;
        let can_north = cam.center_y > bounds.min_center_y + EPS;
        let can_south = cam.center_y < bounds.max_center_y - EPS;
        (can_west, can_east, can_north, can_south)
    }
}
