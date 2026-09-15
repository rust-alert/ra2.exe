//! 对局雷达：开图边沿、俯视小地图、点击跳转。

use ra_layout::rect_px_from_snapshot;
use ra_map::MapEntityKind;
use ra_renderer::{Renderer, RgbaImage};
use ra_widgets::battle_hud::{
    RadarMinimapBlip, compose_radar_minimap, radar_content_rect, radar_fit_xy_to_cell, radar_minimap_fit_rect, radar_open_animation_done,
};
use winit::window::Window;

use super::BattleController;

impl BattleController {
    /// 当前本机雷达是否应开图（存活雷达且未低电）。
    pub(super) fn local_radar_online(&self) -> bool {
        let Some(game) = self.session.as_ref().and_then(|s| s.battle())
        else {
            return false;
        };
        let local_id = game.world.local_player;
        let Some(local) = game.world.players.iter().find(|p| p.id == local_id)
        else {
            return false;
        };
        if local.low_power() {
            return false;
        }
        game.snapshot_capabilities(&[]).has_radar
    }

    /// 开图动画是否已结束（可叠小地图 / 接受点击跳转）。
    pub(super) fn radar_content_ready(&self) -> bool {
        if !self.radar_online_latched {
            return false;
        }
        let frame_count = self.hud_chrome.as_ref().map(|c| c.radar_open.len()).unwrap_or(0);
        let Some(started) = self.radar_open_started_tick
        else {
            return true;
        };
        let tick = self.session.as_ref().and_then(|s| s.battle()).map(|g| g.world.tick).unwrap_or(0);
        radar_open_animation_done(started, tick, frame_count)
    }

    /// 边沿检测：上升播 `RadarOn` 并记下开图起点，下降播 `RadarOff` 并清小地图。
    pub(super) fn sync_radar_online_edge(&mut self) {
        let online = self.local_radar_online();
        let tick = self.session.as_ref().and_then(|s| s.battle()).map(|g| g.world.tick).unwrap_or(0);
        if online == self.radar_online_latched {
            return;
        }
        if online {
            self.radar_online_latched = true;
            self.radar_open_started_tick = Some(tick);
            self.queue_battle_sfx_once("RadarOn");
            tracing::info!(tick, "雷达开图");
        }
        else {
            self.radar_online_latched = false;
            self.radar_open_started_tick = None;
            self.radar_minimap = None;
            self.queue_battle_sfx_once("RadarOff");
            tracing::info!(tick, "雷达关图");
        }
    }

    /// 刷新俯视小地图缓存（仅开图后）。
    pub(super) fn refresh_radar_minimap(&mut self, renderer: &Renderer, window: Option<&Window>) {
        if !self.radar_content_ready() {
            self.radar_minimap = None;
            return;
        }
        let view = window.and_then(|w| self.radar_view_cell_rect(renderer, w));
        let Some(image) = self.compose_radar_minimap_image(view)
        else {
            self.radar_minimap = None;
            return;
        };
        self.radar_minimap = Some(image);
    }

    /// 由通行格陆地类型与实体色点合成俯视小地图。
    pub(super) fn compose_radar_minimap_image(&self, view: Option<(u16, u16, u16, u16)>) -> Option<RgbaImage> {
        let game = self.session.as_ref()?.battle()?;
        let grid = &game.world.pass_grid;
        let w = grid.width.max(1);
        let h = grid.height.max(1);
        let mut land = vec![0u8; (w as usize).saturating_mul(h as usize)];
        for y in 0..h {
            for x in 0..w {
                let i = (y * w + x) as usize;
                land[i] = grid.land_type(x as u16, y as u16) as u8;
            }
        }
        let mut blips = Vec::new();
        for id in game.world.entity_ids() {
            let Some((_, _, dead)) = game.world.ecs_health(id)
            else {
                continue;
            };
            if dead {
                continue;
            }
            let Some((tx, ty, _)) = game.world.ecs_transform(id)
            else {
                continue;
            };
            let Some(owner) = game.world.ecs_owner(id)
            else {
                continue;
            };
            let Some((_, kind)) = game.world.ecs_identity(id)
            else {
                continue;
            };
            let rgba = self
                .lobby_primaries
                .get(owner.as_ref())
                .map(|c| [c.r, c.g, c.b, 255])
                .unwrap_or([220, 220, 220, 255]);
            let structure = matches!(kind, MapEntityKind::Structure);
            blips.push(RadarMinimapBlip { x: tx, y: ty, rgba, structure });
        }
        compose_radar_minimap(w, h, &land, &blips, view)
    }

    /// 战术区四角对应的地图格 AABB（画镜头框）。
    pub(super) fn radar_view_cell_rect(&self, renderer: &Renderer, window: &Window) -> Option<(u16, u16, u16, u16)> {
        let game = self.session.as_ref()?.battle()?;
        let vp = self.map_viewport(window);
        let cam = renderer.camera();
        let corners = [
            (vp.tactical.x as f32, vp.tactical.y as f32),
            ((vp.tactical.x + vp.tactical.w) as f32, vp.tactical.y as f32),
            (vp.tactical.x as f32, (vp.tactical.y + vp.tactical.h) as f32),
            ((vp.tactical.x + vp.tactical.w) as f32, (vp.tactical.y + vp.tactical.h) as f32),
        ];
        let mut cells = Vec::new();
        for (sx, sy) in corners {
            let (wx, wy) = vp.screen_to_world(cam, sx, sy);
            if let Some(cell) = game.image_to_cell(wx, wy) {
                cells.push(cell);
            }
        }
        if cells.is_empty() {
            return None;
        }
        let mut x0 = cells[0].0;
        let mut y0 = cells[0].1;
        let mut x1 = x0;
        let mut y1 = y0;
        for (x, y) in cells {
            x0 = x0.min(x);
            y0 = y0.min(y);
            x1 = x1.max(x);
            y1 = y1.max(y);
        }
        Some((x0, y0, x1, y1))
    }

    /// 点击雷达小地图：跳转镜头到对应格。
    pub(super) fn focus_radar_click(&self, renderer: &mut Renderer, window: &Window, px: i32, py: i32) {
        if !self.radar_content_ready() {
            return;
        }
        let Some(thumb) = self.radar_minimap.as_ref()
        else {
            return;
        };
        let snap = self.hud_snap_for_window(window);
        let slot = rect_px_from_snapshot(&snap, "radar");
        let content = radar_content_rect(slot);
        let fit = radar_minimap_fit_rect(thumb.width(), thumb.height(), content);
        let Some((cx, cy)) = radar_fit_xy_to_cell(fit, thumb.width(), thumb.height(), px, py)
        else {
            return;
        };
        self.focus_camera_on_cell(renderer, cx, cy);
        tracing::info!(cx, cy, "雷达跳转");
    }
}
