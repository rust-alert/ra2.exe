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
    pub(super) fn clear_pause_menu_input(&mut self) {
        self.pause_hover = None;
        self.pause_pressed = None;
        self.leave_armed = false;
        self.command_hover = None;
        self.command_pressed = None;
        self.left_gesture = LeftGesture::Idle;
    }

    pub(super) fn pause_hud_metrics(&self) -> BattleHudChromeMetrics {
        self.hud_chrome
            .as_ref()
            .map(|c| BattleHudChromeMetrics::for_mix(&c.mix))
            .or_else(|| self.pause_menu_chrome.as_ref().map(|c| BattleHudChromeMetrics::for_mix(&c.mix)))
            .unwrap_or_else(BattleHudChromeMetrics::sidec01)
    }

    pub(super) fn refresh_pause_hover(&mut self, window: &Window) {
        let size = window.inner_size();
        let metrics = self.pause_hud_metrics();
        self.pause_hover =
            battle_pause_menu::hit_at(size.width.max(1), size.height.max(1), metrics, self.cursor.0 as i32, self.cursor.1 as i32)
                .map(|h| h.entry_id());
    }

    pub(super) fn handle_pause_menu_mouse(&mut self, state: ElementState, window: &Window) -> BattleNav {
        let size = window.inner_size();
        let metrics = self.pause_hud_metrics();
        let x = self.cursor.0 as i32;
        let y = self.cursor.1 as i32;
        match state {
            ElementState::Pressed => {
                self.pause_pressed = battle_pause_menu::hit_at(size.width.max(1), size.height.max(1), metrics, x, y).map(|h| h.entry_id());
                BattleNav::None
            }
            ElementState::Released => {
                let pressed = self.pause_pressed.take();
                let hit = battle_pause_menu::hit_at(size.width.max(1), size.height.max(1), metrics, x, y);
                if pressed.is_some_and(|id| hit.is_some_and(|h| h.entry_id() == id)) {
                    if let Some(hit) = hit {
                        return self.on_pause_menu_hit(hit);
                    }
                }
                BattleNav::None
            }
        }
    }

    pub(super) fn on_pause_menu_hit(&mut self, hit: BattlePauseMenuHit) -> BattleNav {
        match hit {
            BattlePauseMenuHit::Resume => {
                if let Some(game) = self.session.as_mut().and_then(|s| s.battle_mut()) {
                    if game.paused {
                        game.toggle_pause();
                    }
                }
                self.clear_pause_menu_input();
                tracing::info!("继续");
                BattleNav::None
            }
            BattlePauseMenuHit::Abort => {
                self.clear_pause_menu_input();
                if let Some(game) = self.session.as_mut().and_then(|s| s.battle_mut()) {
                    game.apply_scripted_outcome(BattleOutcome::Defeat { reason: "放弃任务".into() });
                    // 关掉暂停菜单输入路径；仿真仍因 `outcome` 停住。
                    if game.paused {
                        game.toggle_pause();
                    }
                }
                // 留在对局页播 EVA，由 `pump` → `poll_outcome_nav` 延后进结算。
                self.begin_outcome_hold();
                tracing::info!("放弃任务 · 先播报再结算");
                BattleNav::None
            }
            BattlePauseMenuHit::Options => {
                self.clear_pause_menu_input();
                tracing::info!("暂停菜单 · 打开选项");
                BattleNav::OpenOptions
            }
            BattlePauseMenuHit::Fullscreen => {
                tracing::info!("暂停菜单 · 切换全屏");
                BattleNav::ToggleFullscreen
            }
        }
    }
}
