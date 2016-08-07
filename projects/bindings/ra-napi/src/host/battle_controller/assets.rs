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

use super::BattleController;


impl BattleController {
    pub(super) fn local_house_name(&self) -> Option<String> {
        self.session
            .as_ref()
            .and_then(|s| s.battle())
            .and_then(|g| g.world.players.iter().find(|p| p.id == g.world.local_player))
            .map(|p| p.house.to_string())
    }

    /// 装入 `mouse.shp` 移动 / 攻击 / 部署命令图标（每局一次）。
    pub(super) fn ensure_order_icons(&mut self, renderer: &mut Renderer, assets: Option<&GameAssetSource>) {
        if self.order_icons_loaded {
            return;
        }
        self.order_icons_loaded = true;
        let Some(source) = assets
        else {
            return;
        };
        match load_battle_order_icons(source) {
            Some(icons) => {
                tracing::info!(
                    "命令图标 · move#{} attack#{} deploy#{} · {}x{}",
                    icons.move_frames.len(),
                    icons.attack_frames.len(),
                    icons.deploy_frames.len(),
                    icons.canvas_w,
                    icons.canvas_h
                );
                renderer.set_order_icons(icons);
            }
            None => tracing::warn!("命令图标装入失败 · 缺少 mouse.shp / mousepal.pal"),
        }
    }

    /// 装入 `pips.shp` / `pipbrd.shp` 选中血条（每局一次）。
    pub(super) fn ensure_selection_overlay(&mut self, renderer: &mut Renderer, assets: Option<&GameAssetSource>) {
        if self.selection_overlay_loaded {
            return;
        }
        self.selection_overlay_loaded = true;
        self.refresh_condition_thresholds(assets);
        let Some(source) = assets
        else {
            return;
        };
        match load_selection_overlay(source) {
            Some(overlay) => {
                tracing::info!(
                    "选中血条 · pipbrd#{} pips#{} · yellow={:.2} red={:.2}",
                    overlay.pipbrd_frames.len(),
                    overlay.pips_frames.len(),
                    self.condition_yellow,
                    self.condition_red
                );
                renderer.set_selection_overlay(overlay);
            }
            None => tracing::warn!("选中血条装入失败 · 缺少 pips.shp / pipbrd.shp / palette.pal"),
        }
    }

    /// 从 rules 刷新 `ConditionYellow` / `ConditionRed`。
    pub(super) fn refresh_condition_thresholds(&mut self, assets: Option<&GameAssetSource>) {
        use ra_types::AssetSource;
        let Some(source) = assets
        else {
            return;
        };
        let Ok(bytes) = source.read(self.rules_ini)
        else {
            return;
        };
        let Ok(doc) = IniDocument::parse(&bytes)
        else {
            return;
        };
        let damage = ra_map::StructureDamageRules::from_rules_doc(&doc);
        self.condition_yellow = damage.yellow;
        self.condition_red = damage.red;
    }

    /// 按本地阵营解码侧栏/底栏 chrome（仅在缺失或换边时重解）。
    pub(super) fn ensure_battle_hud_chrome(&mut self, assets: Option<&GameAssetSource>) {
        let Some(source) = assets
        else {
            return;
        };
        let Some(side) = self.local_house_name()
        else {
            return;
        };
        if self.hud_chrome.as_ref().is_some_and(|c| c.side == side) {
            return;
        }
        let chrome = decode_battle_hud_chrome_with(source, &side, self.ui_faction_side.as_deref(), self.ui_faction_chrome.as_ref());
        if chrome.has_sidebar_body() {
            let pal_origin =
                chrome.side1.as_ref().or(chrome.side2.as_ref()).or(chrome.credits.as_ref()).map(|s| s.origin.as_str()).unwrap_or("-");
            tracing::info!(
                side = %chrome.side,
                faction = ?self.ui_faction_side,
                mix = %chrome.mix,
                pal_origin,
                errors = chrome.errors.len(),
                "对局 HUD chrome 已解码"
            );
        } else {
            tracing::warn!(
                side = %side,
                mix = %chrome.mix,
                errors = ?chrome.errors,
                "对局 HUD chrome 未解出侧栏主体，回退占位条"
            );
        }
        self.hud_chrome = Some(chrome);
    }

    /// 按本地阵营解码暂停菜单素材（换边重解；必须 prefer `sidec*`）。
    pub(super) fn ensure_pause_menu_chrome(&mut self, assets: Option<&GameAssetSource>) {
        let Some(side) = self.local_house_name()
        else {
            return;
        };
        if self.pause_menu_tried_side.as_deref() == Some(side.as_str()) {
            return;
        }
        self.pause_menu_tried_side = Some(side.clone());
        let Some(source) = assets
        else {
            self.pause_menu_chrome = None;
            return;
        };
        let decoded =
            battle_pause_menu::decode_battle_pause_chrome_with(source, &side, self.ui_faction_side.as_deref(), self.ui_faction_chrome.as_ref());
        if !decoded.errors.is_empty() {
            tracing::warn!(side = %side, mix = %decoded.mix, errors = ?decoded.errors, "暂停菜单素材有缺口");
        } else {
            tracing::info!(side = %side, mix = %decoded.mix, "暂停菜单素材已解码");
        }
        self.pause_menu_chrome = Some(decoded);
    }

    pub(super) fn ensure_cameo_cache(&mut self, assets: Option<&GameAssetSource>) {
        let Some(source) = assets
        else {
            return;
        };
        let Some(caps) = self.current_capabilities()
        else {
            return;
        };
        let art = source.resolve(self.art_ini).and_then(|hit| IniDocument::parse(&hit.bytes).ok());
        let art_ref = art.as_ref();
        for item in caps
            .build_items
            .iter()
            .chain(caps.defense_items.iter())
            .chain(caps.infantry_items.iter())
            .chain(caps.vehicle_items.iter())
            .chain(caps.aircraft_items.iter())
        {
            let key = item.type_id.as_ref();
            if self.cameo_cache.contains_key(key) {
                continue;
            }
            let sprite = decode_cameo_sprite(source, art_ref, key);
            self.cameo_cache.insert(key.to_string(), sprite);
        }
    }
}
