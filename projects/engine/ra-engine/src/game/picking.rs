use crate::state::components::{Health, Identity, Owner, Transform};
use ra_map::{MapEntityKind, iso_to_screen, screen_to_iso};
use ra_types::EntityId;

use super::session::BattleSession;

impl BattleSession {
    /// 预览图像素 → 地图格（粗逆变换，再用格高修正一次）。
    pub fn image_to_cell(&self, image_x: f32, image_y: f32) -> Option<(u16, u16)> {
        let px = image_x.round() as i32 + self.preview_origin_x;
        let py = image_y.round() as i32 + self.preview_origin_y;
        let (rx0, ry0) = screen_to_iso(px, py, 0);
        if rx0 < 0 || ry0 < 0 {
            return None;
        }
        let x0 = rx0 as u16;
        let y0 = ry0 as u16;
        if !self.world.pass_grid.in_bounds(x0, y0) {
            return None;
        }
        let z = self.world.pass_grid.cell_height(x0, y0);
        let (rx, ry) = screen_to_iso(px, py, z);
        if rx < 0 || ry < 0 {
            return None;
        }
        let x = rx as u16;
        let y = ry as u16;
        if !self.world.pass_grid.in_bounds(x, y) {
            return None;
        }
        Some((x, y))
    }

    /// 点选格上或其四邻的存活移动单位。
    pub fn pick_mobile_at(&self, x: u16, y: u16) -> Option<EntityId> {
        self.pick_mobile_at_owned(x, y, None)
    }

    /// 点选格上或其四邻的存活移动单位；`owner` 若给出则只匹配该阵营（本方点选）。
    pub fn pick_mobile_at_owned(&self, x: u16, y: u16, owner: Option<&str>) -> Option<EntityId> {
        let mut best: Option<(u32, EntityId)> = None;
        for e in &self.world.entities {
            let id = e.id;
            if self.world.ecs_get::<Health>(id).map(|h| h.dead).unwrap_or(true) {
                continue;
            }
            if !self
                .world
                .ecs_get::<Identity>(id)
                .map(|identity| matches!(identity.kind, MapEntityKind::Unit | MapEntityKind::Infantry | MapEntityKind::Aircraft))
                .unwrap_or(false)
            {
                continue;
            }
            if let Some(want) = owner {
                let Some(o) = self.world.ecs_get::<Owner>(id)
                else {
                    continue;
                };
                if o.house.as_ref() != want {
                    continue;
                }
            }
            let Some(xf) = self.world.ecs_get::<Transform>(id).copied()
            else {
                continue;
            };
            // VXL/SHP 叠画有像素偏移，点到车身上常落在邻格甚至隔一格。
            let dist = (i32::from(xf.x) - i32::from(x)).unsigned_abs() + (i32::from(xf.y) - i32::from(y)).unsigned_abs();
            if dist > 3 {
                continue;
            }
            if best.map(|(d, _)| dist < d).unwrap_or(true) {
                best = Some((dist, id));
            }
        }
        best.map(|(_, id)| id)
    }

    /// 点选格上或其四邻的存活实体（单位优先，其次建筑）。
    pub fn pick_entity_at(&self, x: u16, y: u16) -> Option<EntityId> {
        self.pick_mobile_at(x, y).or_else(|| self.pick_structure_at(x, y))
    }

    /// 按预览图像素位置点选本地玩家可控制的移动单位（容忍 VXL/SHP 相对格子中心的绘制偏移）。
    ///
    /// `max_dist_px` 为图像空间欧氏距离上限。同距时优先 `MCV` 类型。
    pub fn pick_local_mobile_near_image(&self, image_x: f32, image_y: f32, max_dist_px: f32) -> Option<EntityId> {
        let local_house = self.world.players.iter().find(|p| p.id == self.world.local_player)?.house.clone();
        let mut best: Option<(f32, bool, EntityId)> = None;
        for e in &self.world.entities {
            let id = e.id;
            if self.world.ecs_get::<Health>(id).map(|h| h.dead).unwrap_or(true) {
                continue;
            }
            let Some(owner) = self.world.ecs_get::<Owner>(id)
            else {
                continue;
            };
            if owner.house.as_ref() != local_house.as_ref() {
                continue;
            }
            let Some(identity) = self.world.ecs_get::<Identity>(id)
            else {
                continue;
            };
            if !matches!(identity.kind, MapEntityKind::Unit | MapEntityKind::Infantry | MapEntityKind::Aircraft) {
                continue;
            }
            let Some(xf) = self.world.ecs_get::<Transform>(id).copied()
            else {
                continue;
            };
            let z = self.world.pass_grid.cell_height(xf.x, xf.y);
            let (sx, sy) = iso_to_screen(i32::from(xf.x), i32::from(xf.y), z);
            // 与标记绘制一致：菱形落在格子视觉中心附近。
            let cx = (sx - self.preview_origin_x) as f32 + 30.0;
            let cy = (sy - self.preview_origin_y) as f32 + 15.0;
            let dx = cx - image_x;
            let dy = cy - image_y;
            let dist = (dx * dx + dy * dy).sqrt();
            if dist > max_dist_px {
                continue;
            }
            let is_mcv = identity.type_id.to_ascii_uppercase().contains("MCV");
            let better = match best {
                None => true,
                Some((best_dist, best_mcv, _)) => dist < best_dist - 0.5 || ((dist - best_dist).abs() <= 0.5 && is_mcv && !best_mcv),
            };
            if better {
                best = Some((dist, is_mcv, id));
            }
        }
        best.map(|(_, _, id)| id)
    }

    /// 本地玩家开局移动单位（优先名称含 `MCV` 的载具）。
    pub fn local_start_mobile(&self) -> Option<EntityId> {
        let local_house = self.world.players.iter().find(|p| p.id == self.world.local_player)?.house.clone();
        let mut fallback = None;
        for e in &self.world.entities {
            let id = e.id;
            if self.world.ecs_get::<Health>(id).map(|h| h.dead).unwrap_or(true) {
                continue;
            }
            let Some(owner) = self.world.ecs_get::<Owner>(id)
            else {
                continue;
            };
            if owner.house.as_ref() != local_house.as_ref() {
                continue;
            }
            let Some(identity) = self.world.ecs_get::<Identity>(id)
            else {
                continue;
            };
            if !matches!(identity.kind, MapEntityKind::Unit | MapEntityKind::Infantry | MapEntityKind::Aircraft) {
                continue;
            }
            if identity.type_id.to_ascii_uppercase().contains("MCV") {
                return Some(id);
            }
            if fallback.is_none() {
                fallback = Some(id);
            }
        }
        fallback
    }

    /// 点选落在建筑 `Foundation` 占地内的存活建筑（锚点为左上角格）。
    pub fn pick_structure_at(&self, x: u16, y: u16) -> Option<EntityId> {
        let mut best: Option<(u32, EntityId)> = None;
        for e in &self.world.entities {
            let id = e.id;
            if self.world.ecs_get::<Health>(id).map(|h| h.dead).unwrap_or(true) {
                continue;
            }
            let Some(identity) = self.world.ecs_get::<Identity>(id)
            else {
                continue;
            };
            if identity.kind != MapEntityKind::Structure {
                continue;
            }
            let Some(xf) = self.world.ecs_get::<Transform>(id).copied()
            else {
                continue;
            };
            let (fw, fh) = self
                .world
                .definitions
                .structures
                .get(identity.type_id.as_ref())
                .map(|s| (s.foundation.width.max(1), s.foundation.height.max(1)))
                .unwrap_or((1, 1));
            if x < xf.x || y < xf.y {
                continue;
            }
            let dx = u32::from(x - xf.x);
            let dy = u32::from(y - xf.y);
            if dx >= u32::from(fw) || dy >= u32::from(fh) {
                continue;
            }
            // 重叠时取离锚点更近的占地。
            let dist = dx + dy;
            if best.map(|(d, _)| dist < d).unwrap_or(true) {
                best = Some((dist, id));
            }
        }
        best.map(|(_, id)| id)
    }

    /// 按预览图像素点选本地玩家建筑：命中其 `Foundation` 各格的等距菱形（含向上抬起的主体带）。
    ///
    /// 不用「整栋任意半径圆」；格与格之间也不留圆命中空隙。
    pub fn pick_local_structure_near_image(&self, image_x: f32, image_y: f32) -> Option<EntityId> {
        /// 等距格半宽/半高（与脚点 `+30,+15` 菱形一致）。
        const HALF_W: f32 = 30.0;
        const HALF_H: f32 = 15.0;
        /// 主体相对脚点上抬采样（贴 SHP/建造场立面，非脚底一点）。
        const BODY_LIFTS_PX: &[f32] = &[0.0, 12.0, 24.0, 36.0, 48.0];
        let local_house = self.world.players.iter().find(|p| p.id == self.world.local_player)?.house.clone();
        let mut best: Option<(f32, EntityId)> = None;
        for e in &self.world.entities {
            let id = e.id;
            if self.world.ecs_get::<Health>(id).map(|h| h.dead).unwrap_or(true) {
                continue;
            }
            let Some(owner) = self.world.ecs_get::<Owner>(id)
            else {
                continue;
            };
            if owner.house.as_ref() != local_house.as_ref() {
                continue;
            }
            let Some(identity) = self.world.ecs_get::<Identity>(id)
            else {
                continue;
            };
            if identity.kind != MapEntityKind::Structure {
                continue;
            }
            let Some(xf) = self.world.ecs_get::<Transform>(id).copied()
            else {
                continue;
            };
            let (fw, fh) = self
                .world
                .definitions
                .structures
                .get(identity.type_id.as_ref())
                .map(|s| (s.foundation.width.max(1), s.foundation.height.max(1)))
                .unwrap_or((1, 1));
            let mut min_dist = f32::INFINITY;
            let mut hit = false;
            for oy in 0..fh {
                for ox in 0..fw {
                    let cx = xf.x.saturating_add(ox);
                    let cy = xf.y.saturating_add(oy);
                    let z = self.world.pass_grid.cell_height(cx, cy);
                    let (sx, sy) = iso_to_screen(i32::from(cx), i32::from(cy), z);
                    let foot_x = (sx - self.preview_origin_x) as f32 + 30.0;
                    let foot_y = (sy - self.preview_origin_y) as f32 + 15.0;
                    for &lift in BODY_LIFTS_PX {
                        let px = foot_x;
                        let py = foot_y - lift;
                        let dx = ((image_x - px) / HALF_W).abs();
                        let dy = ((image_y - py) / HALF_H).abs();
                        if dx + dy <= 1.0 {
                            hit = true;
                            let dist = ((image_x - px).powi(2) + (image_y - py).powi(2)).sqrt();
                            min_dist = min_dist.min(dist);
                        }
                    }
                }
            }
            if !hit {
                continue;
            }
            if best.map(|(best_dist, _)| min_dist < best_dist).unwrap_or(true) {
                best = Some((min_dist, id));
            }
        }
        best.map(|(_, id)| id)
    }

    /// 相对 `from` 最近的异阵营存活目标（移动单位或建筑）。
    pub fn nearest_hostile(&self, from: EntityId) -> Option<EntityId> {
        if self.world.ecs_get::<Health>(from).map(|h| h.dead).unwrap_or(true) {
            return None;
        }
        let owner = self.world.ecs_get::<Owner>(from)?.house.clone();
        let xf = self.world.ecs_get::<Transform>(from).copied()?;
        let (fx, fy) = (xf.x, xf.y);
        self.world
            .entities
            .iter()
            .filter_map(|e| {
                let id = e.id;
                if id == from {
                    return None;
                }
                if self.world.ecs_get::<Health>(id).map(|h| h.dead).unwrap_or(true) {
                    return None;
                }
                let identity = self.world.ecs_get::<Identity>(id)?;
                if !matches!(identity.kind, MapEntityKind::Unit | MapEntityKind::Infantry | MapEntityKind::Aircraft | MapEntityKind::Structure)
                {
                    return None;
                }
                let other_owner = self.world.ecs_get::<Owner>(id)?;
                if other_owner.house == owner {
                    return None;
                }
                let ox = self.world.ecs_get::<Transform>(id)?;
                let dx = i32::from(ox.x) - i32::from(fx);
                let dy = i32::from(ox.y) - i32::from(fy);
                Some((dx * dx + dy * dy, id))
            })
            .min_by_key(|(dist, _)| *dist)
            .map(|(_, id)| id)
    }
}
