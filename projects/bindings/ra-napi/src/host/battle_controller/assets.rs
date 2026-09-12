//! 对局页控制器：输入意图、命令、tick、快照；不含窗口与页面导航外壳。

use ra_renderer::Renderer;
use ra_widgets::{
    battle_hud::{decode_battle_hud_chrome_with, decode_cameo_sprite},
    battle_order_icons::load_battle_order_icons,
    battle_pause_menu::{self},
    battle_selection_overlay::load_selection_overlay,
    fs_source::GameAssetSource,
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

    /// 从已缓存受损规则刷新 `ConditionYellow` / `ConditionRed`。
    pub(super) fn refresh_condition_thresholds(&mut self, _assets: Option<&GameAssetSource>) {
        self.condition_yellow = self.paint_ini.damage.yellow;
        self.condition_red = self.paint_ini.damage.red;
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
        }
        else {
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
        }
        else {
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
        let art_ref = self.paint_ini.art.as_ref();
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
