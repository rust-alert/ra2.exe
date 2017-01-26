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
        self.condition_yellow = self.paint.damage.yellow;
        self.condition_red = self.paint.damage.red;
    }

    /// 按本地阵营解码侧栏/底栏 chrome（仅在缺失或换边时重解）。
    pub(crate) fn ensure_battle_hud_chrome(&mut self, assets: Option<&GameAssetSource>) {
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
    pub(crate) fn ensure_pause_menu_chrome(&mut self, assets: Option<&GameAssetSource>) {
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
            let names = self.paint.cameo_asset_names(key);
            let sprite = decode_cameo_sprite(source, &names);
            self.cameo_cache.insert(key.to_string(), sprite);
        }
    }

    /// 胜负收束期：战役优先解码 `CampaignScore.Animation`（调色板 `CampaignScore.Palette`）。
    pub(super) fn ensure_outcome_banner(&mut self, assets: Option<&GameAssetSource>) {
        if self.outcome_banner_tried {
            return;
        }
        let Some(game) = self.session.as_ref().and_then(|s| s.battle())
        else {
            return;
        };
        // 收束窗内已有暂存胜负，即可解码横幅（不必等 `outcome` 落盘）。
        if game.outcome.is_none() && game.pending_savour_outcome.is_none() {
            return;
        }
        self.outcome_banner_tried = true;
        if game.boot_kind != ra_engine::SessionBootKind::Campaign {
            return;
        }
        let Some(source) = assets
        else {
            return;
        };
        let Some(chrome) = self.ui_faction_chrome.as_ref()
        else {
            return;
        };
        let pals = ra_widgets::skirmish_setup::campaign_score_screen_palette_candidates(chrome);
        let anims = ra_widgets::skirmish_setup::campaign_score_screen_animation_candidates(chrome);
        let Some(pal) = pals.into_iter().find(|p| source.resolve(p).is_some())
        else {
            tracing::warn!("战役收束横幅缺可读 CampaignScore.Palette");
            return;
        };
        for anim in anims {
            if source.resolve(&anim).is_none() {
                continue;
            }
            let asset = ra_widgets::screens::page::UiAssetRef::with_palette(&anim, &pal);
            match ra_widgets::skin::decode::decode_asset_frames(source, &asset) {
                Ok(frames) if !frames.is_empty() => {
                    tracing::info!(%anim, %pal, frames = frames.len(), "战役收束横幅全帧已解码");
                    self.outcome_banner_frames = frames;
                    self.outcome_banner_frame = 0;
                    self.outcome_banner_clock = None;
                    self.outcome_banner_accum = 0.0;
                    return;
                }
                Ok(_) => tracing::warn!(%anim, "CampaignScore.Animation 无帧"),
                Err(e) => tracing::warn!(%anim, %pal, "CampaignScore.Animation 解码失败 · {e}"),
            }
        }
    }

    /// 当前应绘制的收束横幅帧；播完后停在末帧。
    pub(super) fn outcome_banner_sprite(&self) -> Option<&ra_widgets::skin::decode::DecodedUiSprite> {
        if self.outcome_banner_frames.is_empty() {
            return None;
        }
        let last = self.outcome_banner_frames.len() - 1;
        self.outcome_banner_frames.get(self.outcome_banner_frame.min(last))
    }

    /// 战役收束横幅 10 FPS；未播完才进帧。
    pub(super) fn tick_outcome_banner_anim(&mut self) {
        let n = self.outcome_banner_frames.len();
        if n <= 1 || self.outcome_banner_frame >= n - 1 {
            self.outcome_banner_clock = None;
            return;
        }
        const FRAME_SECS: f64 = 0.1;
        let dt = self.outcome_banner_clock.replace(std::time::Instant::now()).map(|t0| t0.elapsed().as_secs_f64()).unwrap_or(0.0).min(0.25);
        self.outcome_banner_accum += dt;
        while self.outcome_banner_accum >= FRAME_SECS && self.outcome_banner_frame < n - 1 {
            self.outcome_banner_accum -= FRAME_SECS;
            self.outcome_banner_frame += 1;
        }
        if self.outcome_banner_frame >= n - 1 {
            self.outcome_banner_clock = None;
            self.outcome_banner_accum = 0.0;
        }
    }
}
