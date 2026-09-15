//! 对局页控制器：输入意图、命令、tick、快照；不含窗口与页面导航外壳。

use std::{collections::HashMap, time::Instant};

use ra_map::{
    MapEntity, MapEntityKind, MobilePaintPose, StructureBuildupClip, collect_structure_anim_bank, load_structure_buildup_clip,
    load_structure_erase_masks, paint_mobiles_onto_preview_rgba, paint_structure_anims_onto_rgba, paint_structure_buildup_onto_rgba,
    paint_structures_onto_rgba, paint_terrain_anims_onto_rgba, restore_structure_blit_from_ground, restore_structure_foundation_from_ground,
};
use ra_renderer::Renderer;
use ra_types::{EntityId, HouseName, TechnoName};
use ra_widgets::fs_source::GameAssetSource;

use super::super::boot::remap_owner_palette;

use super::{BattleController, movement::mobile_paint_pose_for};

/// 待播的建筑 Buildup（MCV 展开等）。
pub(super) struct PendingBuildup {
    #[allow(dead_code)]
    entity: EntityId,
    type_id: String,
    owner: String,
    clip: StructureBuildupClip,
    started: Instant,
}

/// 部署已在权威侧完成、等待呈现侧播动画的任务。
pub(super) struct DeployVisualJob {
    entity: EntityId,
    type_id: String,
    owner: String,
    x: u16,
    y: u16,
}

/// 待播的建筑拆除 / 出售倒放。
pub(super) struct PendingTeardown {
    #[allow(dead_code)]
    entity: EntityId,
    type_id: String,
    clip: Option<StructureBuildupClip>,
    started: Instant,
    x: u16,
    y: u16,
}

/// 权威拆除完成后等待呈现侧播动画的任务。
pub(in crate::host) struct TeardownVisualJob {
    entity: EntityId,
    type_id: String,
    owner: String,
    x: u16,
    y: u16,
    foundation_w: u16,
    foundation_h: u16,
}

impl BattleController {
    /// 对当前选中立即下发部署（悬停点选 / 命令条 Deploy / `D`；无单独工具态）。
    pub(super) fn deploy_selection(&mut self) {
        let selected = self.local.selected.clone();
        let Some(game) = self.session.as_ref().and_then(|s| s.battle())
        else {
            return;
        };
        let Some(&id) = selected.iter().find(|&&eid| game.entity_can_deploy(eid))
        else {
            tracing::info!("部署 · 选中无可部署单位 · {:?}", selected);
            return;
        };
        self.deploy_watch = Some(id);
        self.follow_mode = false;
        self.attack_move_mode = false;
        if self.planning_mode {
            self.planning_mode = false;
            self.planning_waypoints.clear();
        }
        if let Some(game) = self.session.as_mut().and_then(|s| s.battle_mut()) {
            tracing::info!("部署选中 · {:?}", selected);
            game.order_deploy(&selected);
        }
    }

    /// 对当前选中下发就地警戒（`G` / 命令条 Guard；`keyboard.ini` GuardObject）。
    pub(super) fn guard_selection(&mut self) {
        let selected = self.local.selected.clone();
        if selected.is_empty() {
            return;
        }
        if let Some(game) = self.session.as_mut().and_then(|s| s.battle_mut()) {
            tracing::info!("警戒选中 · {:?}", selected);
            game.order_guard(&selected);
        }
    }

    /// 对当前选中下发停止（`S` / 命令条 Stop；`keyboard.ini` StopObject）。右键不清停，只取消选中。
    pub(super) fn stop_selection(&mut self) {
        let selected = self.local.selected.clone();
        if selected.is_empty() {
            return;
        }
        if let Some(game) = self.session.as_mut().and_then(|s| s.battle_mut()) {
            tracing::info!("停止选中 · {:?}", selected);
            game.order_stop(&selected);
        }
    }

    /// 对当前选中下发散开（`X` / `keyboard.ini` ScatterObject）。
    pub(super) fn scatter_selection(&mut self) {
        let selected = self.local.selected.clone();
        if selected.is_empty() {
            return;
        }
        let pulse_tick = self.session.as_mut().and_then(|s| s.battle_mut()).map(|game| {
            tracing::info!("散开选中 · {:?}", selected);
            let tick = game.world.tick;
            game.order_scatter(&selected);
            tick
        });
        if let Some(tick) = pulse_tick {
            self.pulse_action_lines_at(tick);
        }
    }

    /// 删除当前选中（`Delete` 热键；无退款）。
    pub(super) fn delete_selection(&mut self) {
        let selected = self.local.selected.clone();
        if selected.is_empty() {
            return;
        }
        if let Some(game) = self.session.as_mut().and_then(|s| s.battle_mut()) {
            tracing::info!("删除选中 · {:?}", selected);
            game.order_delete(&selected);
        }
        self.local.clear();
    }

    /// 切换攻击移动模式（命令条 AttackMove / 可选热键）。
    pub(super) fn toggle_attack_move_mode(&mut self) {
        if self.local.selected.is_empty() {
            tracing::info!("攻击移动 · 无选中单位，忽略");
            return;
        }
        self.place_mode = None;
        self.repair_mode = false;
        self.sell_mode = false;
        self.follow_mode = false;
        if self.planning_mode {
            self.planning_mode = false;
            self.planning_waypoints.clear();
        }
        self.attack_move_mode = !self.attack_move_mode;
        tracing::info!(active = self.attack_move_mode, "命令条 · 攻击移动");
    }

    /// 切换跟随模式（热键 Follow；左键点选目标下发 `order_follow`）。
    pub(super) fn toggle_follow_mode(&mut self) {
        if self.local.selected.is_empty() {
            tracing::info!("跟随 · 无选中单位，忽略");
            return;
        }
        let has_mobile = self.session.as_ref().and_then(|s| s.battle()).is_some_and(|game| {
            self.local.selected.iter().any(|&id| {
                game.world
                    .ecs_identity(id)
                    .is_some_and(|(_, kind)| matches!(kind, MapEntityKind::Unit | MapEntityKind::Infantry | MapEntityKind::Aircraft))
            })
        });
        if !has_mobile {
            tracing::info!("跟随 · 选中无机动单位，忽略");
            self.follow_mode = false;
            return;
        }
        self.place_mode = None;
        self.repair_mode = false;
        self.sell_mode = false;
        self.attack_move_mode = false;
        if self.planning_mode {
            self.planning_mode = false;
            self.planning_waypoints.clear();
        }
        self.follow_mode = !self.follow_mode;
        tracing::info!(active = self.follow_mode, "命令条 · 跟随模式");
    }

    /// 根据权威世界更新部署中 / 完成 / 拒绝状态。
    pub(super) fn resolve_deploy_watch(&mut self) {
        let Some(id) = self.deploy_watch
        else {
            return;
        };
        let resolved = {
            let Some(game) = self.session.as_ref().and_then(|s| s.battle())
            else {
                self.deploy_watch = None;
                return;
            };
            if game.world.last_rejects().iter().any(|r| matches!(r.reason, ra_engine::CommandRejectReason::CannotDeploy)) {
                Some(Err(ra_engine::CommandRejectReason::CannotDeploy.as_hud_label().to_string()))
            }
            else {
                match game.world.ecs_identity(id) {
                    Some((type_id, kind)) if matches!(kind, MapEntityKind::Structure) => Some(Ok(type_id.to_string())),
                    None => Some(Err("部署目标已消失".into())),
                    _ => None,
                }
            }
        };
        match resolved {
            Some(Ok(type_id)) => {
                tracing::info!("部署完成 · {type_id} · #{id}", id = id.0);
                self.deploy_watch = None;
                // Buildup 由权威 `structure_buildup_dirty` 驱动，避免与本机队列重复入队。
            }
            Some(Err(label)) => {
                tracing::info!("部署失败 · {label}");
                self.deploy_watch = None;
            }
            None => {}
        }
    }

    /// 启动 / 推进部署 Buildup 与拆除倒放，并在播放期间重绘预览。
    pub(super) fn tick_deploy_visuals(&mut self, assets: Option<&GameAssetSource>, renderer: &mut Renderer) {
        self.enqueue_structure_buildup_jobs();
        self.enqueue_structure_teardown_jobs();
        let Some(assets) = assets
        else {
            return;
        };
        let had_deploy_queue = !self.deploy_visual_queue.is_empty();
        let had_teardown_queue = !self.teardown_visual_queue.is_empty();
        let jobs: Vec<DeployVisualJob> = self.deploy_visual_queue.drain(..).collect();
        let teardown_jobs: Vec<TeardownVisualJob> = self.teardown_visual_queue.drain(..).collect();
        let pending_before = self.pending_buildups.len();
        let teardown_before = self.pending_teardowns.len();
        for job in jobs {
            self.begin_deploy_visual(assets, job);
        }
        for job in teardown_jobs {
            self.begin_teardown_visual(assets, job);
        }
        let started_new = self.pending_buildups.len() > pending_before || self.pending_teardowns.len() > teardown_before;
        if self.pending_buildups.is_empty() && self.pending_teardowns.is_empty() {
            if had_deploy_queue || had_teardown_queue {
                // 无动画资源时已定格 / 已擦除：必须上传底图。
                self.present_preview_base(renderer);
            }
            return;
        }
        if started_new {
            self.recompose_preview_with_structure_fx(assets, renderer);
        }
        let mut still = Vec::new();
        let mut finished = Vec::new();
        for pending in self.pending_buildups.drain(..) {
            let elapsed = pending.started.elapsed().as_millis() as u64;
            if pending.clip.frame_at(elapsed).is_none() {
                finished.push(pending);
            }
            else {
                still.push(pending);
            }
        }
        self.pending_buildups = still;
        for done in &finished {
            tracing::info!("部署动画结束 · {} @({},{})", done.type_id, done.clip.x, done.clip.y);
            self.settle_deployed_structure(assets, &done.type_id, &done.owner, done.clip.x, done.clip.y, Some(&done.clip));
        }

        let mut still_teardown = Vec::new();
        let mut finished_teardown = Vec::new();
        for pending in self.pending_teardowns.drain(..) {
            let elapsed = pending.started.elapsed().as_millis() as u64;
            let done = match pending.clip.as_ref() {
                Some(clip) => clip.frame_at_reverse(elapsed).is_none(),
                None => true,
            };
            if done {
                finished_teardown.push(pending);
            }
            else {
                still_teardown.push(pending);
            }
        }
        self.pending_teardowns = still_teardown;
        for done in &finished_teardown {
            tracing::info!("拆除动画结束 · {} @({},{})", done.type_id, done.x, done.y);
        }
        if !finished_teardown.is_empty() {
            self.rebuild_preview_base_with_mobiles(assets);
        }

        if self.pending_buildups.is_empty() && self.pending_teardowns.is_empty() {
            self.present_preview_base(renderer);
        }
        else {
            self.recompose_preview_with_structure_fx(assets, renderer);
        }
    }

    /// 将权威侧新建建筑脏集转入 Buildup 呈现队列（放置 / 部署共用）。
    pub(super) fn enqueue_structure_buildup_jobs(&mut self) {
        let dirty = self.session.as_mut().and_then(|s| s.battle_mut()).map(|g| g.world.take_structure_buildup_dirty()).unwrap_or_default();
        if dirty.is_empty() {
            return;
        }
        let Some(game) = self.session.as_ref().and_then(|s| s.battle())
        else {
            return;
        };
        for id in dirty {
            if game.world.ecs_health(id).map(|(_, _, dead)| dead).unwrap_or(true) {
                continue;
            }
            let Some((type_id, kind)) = game.world.ecs_identity(id)
            else {
                continue;
            };
            if kind != MapEntityKind::Structure {
                continue;
            }
            let Some(owner) = game.world.ecs_owner(id)
            else {
                continue;
            };
            let Some((x, y, _)) = game.world.ecs_transform(id)
            else {
                continue;
            };
            if self.deploy_visual_queue.iter().any(|j| j.entity == id) || self.pending_buildups.iter().any(|p| p.entity == id) {
                continue;
            }
            self.deploy_visual_queue.push(DeployVisualJob { entity: id, type_id: type_id.to_string(), owner: owner.to_string(), x, y });
        }
    }

    /// 将权威侧拆除 / 出售脏集转入倒放呈现队列。
    pub(super) fn enqueue_structure_teardown_jobs(&mut self) {
        let dirty = self.session.as_mut().and_then(|s| s.battle_mut()).map(|g| g.world.take_structure_teardown_dirty()).unwrap_or_default();
        if dirty.is_empty() {
            return;
        }
        let Some(game) = self.session.as_ref().and_then(|s| s.battle())
        else {
            return;
        };
        for id in dirty {
            let Some((type_id, kind)) = game.world.ecs_identity(id)
            else {
                continue;
            };
            if kind != MapEntityKind::Structure {
                continue;
            }
            let Some(owner) = game.world.ecs_owner(id)
            else {
                continue;
            };
            let Some((x, y, _)) = game.world.ecs_transform(id)
            else {
                continue;
            };
            if self.teardown_visual_queue.iter().any(|j| j.entity == id) || self.pending_teardowns.iter().any(|p| p.entity == id) {
                continue;
            }
            // 若同一实体还在播 Buildup，取消展开以免与拆除打架。
            self.pending_buildups.retain(|p| p.entity != id);
            self.deploy_visual_queue.retain(|j| j.entity != id);
            let foundation = game
                .world
                .definitions
                .structures
                .get(type_id.as_ref())
                .map(|s| (s.foundation.width.max(1), s.foundation.height.max(1)))
                .unwrap_or((1, 1));
            self.teardown_visual_queue.push(TeardownVisualJob {
                entity: id,
                type_id: type_id.to_string(),
                owner: owner.to_string(),
                x,
                y,
                foundation_w: foundation.0,
                foundation_h: foundation.1,
            });
        }
    }

    pub(super) fn begin_deploy_visual(&mut self, assets: &GameAssetSource, job: DeployVisualJob) {
        if self.rules.is_none() {
            tracing::warn!("部署动画 · 无规则快照，直接定格 {}", job.type_id);
            self.settle_deployed_structure(assets, &job.type_id, &job.owner, job.x, job.y, None);
            return;
        }
        let clip = {
            let rules = self.rules.as_ref().expect("rules checked");
            let Some(game) = self.session.as_ref().and_then(|s| s.battle())
            else {
                return;
            };
            let lobby = &self.lobby_primaries;
            load_structure_buildup_clip(assets, &game.world.map, &mut self.paint, &job.type_id, &job.owner, job.x, job.y, &|base, owner| {
                remap_owner_palette(rules, Some(lobby), base, owner)
            })
        };
        match clip {
            Some(clip) => {
                tracing::info!("部署动画 · {} @({},{}) · {}帧 · {}ms/帧", job.type_id, job.x, job.y, clip.frames.len(), clip.rate_ms);
                self.pending_buildups.push(PendingBuildup {
                    entity: job.entity,
                    type_id: job.type_id,
                    owner: job.owner,
                    clip,
                    started: Instant::now(),
                });
            }
            None => {
                tracing::warn!("部署动画 · 无 Buildup 资源 {}，尝试直接定格", job.type_id);
                self.settle_deployed_structure(assets, &job.type_id, &job.owner, job.x, job.y, None);
            }
        }
    }

    /// 擦除已烤死的建筑像素，并启动 Buildup 倒放（无资源则立刻结束）。
    pub(super) fn begin_teardown_visual(&mut self, assets: &GameAssetSource, job: TeardownVisualJob) {
        let clip = if let Some(rules) = self.rules.as_ref() {
            let Some(game) = self.session.as_ref().and_then(|s| s.battle())
            else {
                return;
            };
            let lobby = &self.lobby_primaries;
            load_structure_buildup_clip(assets, &game.world.map, &mut self.paint, &job.type_id, &job.owner, job.x, job.y, &|base, owner| {
                remap_owner_palette(rules, Some(lobby), base, owner)
            })
        }
        else {
            None
        };
        let cell_z = clip.as_ref().map(|c| c.cell_z).unwrap_or(0);
        self.erase_structure_from_preview(assets, &job, clip.as_ref(), cell_z);
        self.structure_anims.layers.retain(|layer| !(layer.x == job.x && layer.y == job.y));
        self.last_anim_sig = u64::MAX;
        match clip {
            Some(clip) => {
                tracing::info!("拆除动画 · {} @({},{}) · {}帧倒放 · {}ms/帧", job.type_id, job.x, job.y, clip.frames.len(), clip.rate_ms);
                self.pending_teardowns.push(PendingTeardown {
                    entity: job.entity,
                    type_id: job.type_id,
                    clip: Some(clip),
                    started: Instant::now(),
                    x: job.x,
                    y: job.y,
                });
            }
            None => {
                tracing::info!("拆除 · 无 Buildup，已擦底图 {}", job.type_id);
                self.rebuild_preview_base_with_mobiles(assets);
            }
        }
    }

    /// 从 `preview_clean` / underlay 按精灵遮罩还原无建筑底图像素。
    ///
    /// 优先 Bib+主体 SHP，其次 Buildup 末帧；仅两者皆无时才回退 Foundation 矩形
    /// （矩形会吃掉邻接建筑在屏幕重叠处的像素）。
    fn erase_structure_from_preview(
        &mut self,
        assets: &GameAssetSource,
        job: &TeardownVisualJob,
        clip: Option<&StructureBuildupClip>,
        cell_z: u8,
    ) {
        let origin = self.preview_origin;
        let body_masks = if let Some(rules) = self.rules.as_ref() {
            let Some(game) = self.session.as_ref().and_then(|s| s.battle())
            else {
                return;
            };
            let lobby = &self.lobby_primaries;
            load_structure_erase_masks(assets, &game.world.map, &mut self.paint, &job.type_id, &job.owner, &|base, owner| {
                remap_owner_palette(rules, Some(lobby), base, owner)
            })
        }
        else {
            Vec::new()
        };
        let buildup_mask = clip.and_then(|c| c.frames.last());
        let use_foundation = body_masks.is_empty() && buildup_mask.is_none();

        if let (Some(clean), Some(ground)) = (self.preview_clean.as_mut(), self.preview_structureless_clean.as_ref()) {
            for blit in &body_masks {
                let _ = restore_structure_blit_from_ground(clean, ground, origin.0, origin.1, job.x, job.y, cell_z, blit);
            }
            if body_masks.is_empty() {
                if let Some(blit) = buildup_mask {
                    let _ = restore_structure_blit_from_ground(clean, ground, origin.0, origin.1, job.x, job.y, cell_z, blit);
                }
            }
            if use_foundation {
                let _ = restore_structure_foundation_from_ground(
                    clean,
                    ground,
                    origin.0,
                    origin.1,
                    job.x,
                    job.y,
                    cell_z,
                    job.foundation_w,
                    job.foundation_h,
                );
            }
        }
        if let (Some(underlay), Some(ground)) = (self.preview_ore_underlay.as_mut(), self.preview_structureless_underlay.as_ref()) {
            for blit in &body_masks {
                let _ = restore_structure_blit_from_ground(underlay, ground, origin.0, origin.1, job.x, job.y, cell_z, blit);
            }
            if body_masks.is_empty() {
                if let Some(blit) = buildup_mask {
                    let _ = restore_structure_blit_from_ground(underlay, ground, origin.0, origin.1, job.x, job.y, cell_z, blit);
                }
            }
            if use_foundation {
                let _ = restore_structure_foundation_from_ground(
                    underlay,
                    ground,
                    origin.0,
                    origin.1,
                    job.x,
                    job.y,
                    cell_z,
                    job.foundation_w,
                    job.foundation_h,
                );
            }
        }
        // 遮罩 / 矩形擦除会清掉邻接建筑在屏幕重叠处的像素，必须按深度重烤存活建筑。
        self.repaint_living_structures_onto_preview(assets);
    }

    /// 把仍存活的建筑按深度重叠到 `preview_clean` / underlay（拆除擦除后补邻接残缺）。
    fn repaint_living_structures_onto_preview(&mut self, assets: &GameAssetSource) {
        let Some(rules) = self.rules.as_ref()
        else {
            return;
        };
        let origin = self.preview_origin;
        let lobby = self.lobby_primaries.clone();
        let living: Vec<MapEntity> = {
            let Some(game) = self.session.as_ref().and_then(|s| s.battle())
            else {
                return;
            };
            game.world
                .entity_ids()
                .into_iter()
                .filter_map(|id| {
                    let (cur, max, dead) = game.world.ecs_health(id)?;
                    if dead {
                        return None;
                    }
                    let (type_id, kind) = game.world.ecs_identity(id)?;
                    if kind != MapEntityKind::Structure {
                        return None;
                    }
                    let owner = game.world.ecs_owner(id)?;
                    let (x, y, _) = game.world.ecs_transform(id)?;
                    let health = if max == 0 { 256 } else { ((u64::from(cur) * 256) / u64::from(max)).min(256) as u16 };
                    Some(MapEntity {
                        kind: MapEntityKind::Structure,
                        owner: HouseName::parse(owner.as_ref()),
                        type_id: TechnoName::parse(type_id.as_ref()),
                        health,
                        x,
                        y,
                        facing: 0,
                        sub_cell: 0,
                        mission: Default::default(),
                        tag: Default::default(),
                    })
                })
                .collect()
        };
        if living.is_empty() {
            return;
        }
        let Some(game) = self.session.as_ref().and_then(|s| s.battle())
        else {
            return;
        };
        let mut map = game.world.map.clone();
        map.entities = living;
        let remap = |base: &ra_assets::Palette, own: &str| remap_owner_palette(rules, Some(&lobby), base, own);
        if let Some(clean) = self.preview_clean.as_mut() {
            let _ = paint_structures_onto_rgba(assets, &map, clean, origin.0, origin.1, &mut self.paint, &remap);
        }
        if let Some(underlay) = self.preview_ore_underlay.as_mut() {
            let _ = paint_structures_onto_rgba(assets, &map, underlay, origin.0, origin.1, &mut self.paint, &remap);
        }
        self.last_anim_sig = u64::MAX;
    }

    /// 把已展开建造场烤进 `preview_clean`。主体 SHP 缺失时用 Buildup 末帧。
    pub(super) fn settle_deployed_structure(
        &mut self,
        assets: &GameAssetSource,
        type_id: &str,
        owner: &str,
        x: u16,
        y: u16,
        clip: Option<&StructureBuildupClip>,
    ) {
        let local_house = self
            .session
            .as_ref()
            .and_then(|s| s.battle())
            .and_then(|g| g.world.players.iter().find(|p| p.id == g.world.local_player).map(|p| p.house.to_string()));
        if local_house.as_deref().is_some_and(|house| owner.eq_ignore_ascii_case(house)) {
            self.queue_battle_sfx_once("EVA_ConstructionComplete");
        }
        let origin = self.preview_origin;
        let painted = {
            let Some(rules) = self.rules.as_ref()
            else {
                return;
            };
            let Some(clean) = self.preview_clean.as_mut()
            else {
                return;
            };
            let Some(game) = self.session.as_ref().and_then(|s| s.battle())
            else {
                return;
            };
            let mut one = game.world.map.clone();
            one.entities.clear();
            one.entities.push(MapEntity {
                kind: MapEntityKind::Structure,
                owner: HouseName::parse(owner),
                type_id: TechnoName::parse(type_id),
                health: 256,
                x,
                y,
                facing: 0,
                sub_cell: 0,
                mission: Default::default(),
                tag: Default::default(),
            });
            let lobby = &self.lobby_primaries;
            let mut n = paint_structures_onto_rgba(assets, &one, clean, origin.0, origin.1, &mut self.paint, &|base, own| {
                remap_owner_palette(rules, Some(lobby), base, own)
            });
            if n == 0 {
                if let Some(clip) = clip {
                    if let Some(last) = clip.frames.len().checked_sub(1) {
                        if paint_structure_buildup_onto_rgba(clean, origin.0, origin.1, clip, last) {
                            n = 1;
                            tracing::info!("定格 · {} Buildup 末帧 #{}", type_id, last);
                        }
                    }
                }
            }
            else {
                tracing::info!("定格 · {} 主体 SHP", type_id);
            }
            if n == 0 {
                tracing::warn!("定格失败 · {} 无主体也无 Buildup 帧，保留原预览", type_id);
                return;
            }
            let bank =
                collect_structure_anim_bank(assets, &one, &mut self.paint, &|base, own| remap_owner_palette(rules, Some(lobby), base, own));
            (n, bank)
        };
        let (_n, bank) = painted;
        // underlay 与 clean 同步定格，避免产矿/采集脏刷新丢掉已展开建筑。
        if let (Some(rules), Some(underlay)) = (self.rules.as_ref(), self.preview_ore_underlay.as_mut()) {
            if let Some(game) = self.session.as_ref().and_then(|s| s.battle()) {
                let mut one = game.world.map.clone();
                one.entities.clear();
                one.entities.push(MapEntity {
                    kind: MapEntityKind::Structure,
                    owner: HouseName::parse(owner),
                    type_id: TechnoName::parse(type_id),
                    health: 256,
                    x,
                    y,
                    facing: 0,
                    sub_cell: 0,
                    mission: Default::default(),
                    tag: Default::default(),
                });
                let lobby = &self.lobby_primaries;
                let mut n = paint_structures_onto_rgba(assets, &one, underlay, origin.0, origin.1, &mut self.paint, &|base, own| {
                    remap_owner_palette(rules, Some(lobby), base, own)
                });
                if n == 0 {
                    if let Some(clip) = clip {
                        if let Some(last) = clip.frames.len().checked_sub(1) {
                            if paint_structure_buildup_onto_rgba(underlay, origin.0, origin.1, clip, last) {
                                n = 1;
                            }
                        }
                    }
                }
                let _ = n;
            }
        }
        self.structure_anims.extend_from(bank);
        self.last_anim_sig = u64::MAX;
        // `rules.ini` `[AudioVisual] BuildingSlam=PlaceBuilding`：建造落位 / MCV 展开定格。
        self.pending_battle_sfx.push(super::PendingBattleSfx {
            event: "PlaceBuilding".into(),
            cell: Some((x, y)),
        });
        self.rebuild_preview_base_with_mobiles(assets);
    }

    /// `preview_base` = 已定格底图（含展开后的建造场）+ 当前存活移动单位。
    pub(super) fn rebuild_preview_base_with_mobiles(&mut self, assets: &GameAssetSource) {
        let Some(rules) = self.rules.as_ref()
        else {
            return;
        };
        let Some(clean) = self.preview_clean.as_ref()
        else {
            return;
        };
        let tick_fraction = self.session.as_ref().map(|s| s.tick_fraction()).unwrap_or(0.0);
        let Some(game) = self.session.as_ref().and_then(|s| s.battle())
        else {
            return;
        };
        let mut mobile_map = game.world.map.clone();
        mobile_map.entities.clear();
        let mut poses: HashMap<(u16, u16, TechnoName, HouseName), MobilePaintPose> = HashMap::new();
        for id in game.world.entity_ids() {
            if game.world.ecs_health(id).map(|(_, _, dead)| dead).unwrap_or(true) {
                continue;
            }
            let Some((type_id, kind)) = game.world.ecs_identity(id)
            else {
                continue;
            };
            if !matches!(kind, MapEntityKind::Unit | MapEntityKind::Infantry | MapEntityKind::Aircraft) {
                continue;
            }
            let Some(owner) = game.world.ecs_owner(id)
            else {
                continue;
            };
            let Some((x, y, facing)) = game.world.ecs_transform(id)
            else {
                continue;
            };
            let owner_s = HouseName::parse(owner.as_ref());
            let type_s = TechnoName::parse(type_id.as_ref());
            poses.insert((x, y, type_s.clone(), owner_s.clone()), mobile_paint_pose_for(game, id, x, y, tick_fraction));
            mobile_map.entities.push(MapEntity {
                kind,
                owner: owner_s,
                type_id: type_s,
                health: 256,
                x,
                y,
                facing,
                sub_cell: 0,
                mission: Default::default(),
                tag: Default::default(),
            });
        }
        let mut base = clean.clone();
        let lobby = &self.lobby_primaries;
        paint_mobiles_onto_preview_rgba(
            assets,
            &mobile_map,
            &mut base,
            self.preview_origin.0,
            self.preview_origin.1,
            &mut self.paint,
            &|pal, owner| remap_owner_palette(rules, Some(lobby), pal, owner),
            &|ent| poses.get(&(ent.x, ent.y, ent.type_id.clone(), ent.owner.clone())).copied().unwrap_or_default(),
        );
        self.preview_base = Some(base);
        self.last_anim_sig = u64::MAX;
    }

    /// Buildup / 拆除倒放播放中：干净底图 + 移动单位 + 当前展开或倒放帧 + ActiveAnim。
    pub(super) fn recompose_preview_with_structure_fx(&mut self, assets: &GameAssetSource, renderer: &mut Renderer) {
        let Some(rules) = self.rules.as_ref()
        else {
            return;
        };
        let Some(clean) = self.preview_clean.as_ref()
        else {
            return;
        };
        let tick_fraction = self.session.as_ref().map(|s| s.tick_fraction()).unwrap_or(0.0);
        let Some(game) = self.session.as_ref().and_then(|s| s.battle())
        else {
            return;
        };
        let mut mobile_map = game.world.map.clone();
        mobile_map.entities.clear();
        let mut poses: HashMap<(u16, u16, TechnoName, HouseName), MobilePaintPose> = HashMap::new();
        for id in game.world.entity_ids() {
            if game.world.ecs_health(id).map(|(_, _, dead)| dead).unwrap_or(true) {
                continue;
            }
            let Some((type_id, kind)) = game.world.ecs_identity(id)
            else {
                continue;
            };
            if !matches!(kind, MapEntityKind::Unit | MapEntityKind::Infantry | MapEntityKind::Aircraft) {
                continue;
            }
            let Some(owner) = game.world.ecs_owner(id)
            else {
                continue;
            };
            let Some((x, y, facing)) = game.world.ecs_transform(id)
            else {
                continue;
            };
            let owner_s = HouseName::parse(owner.as_ref());
            let type_s = TechnoName::parse(type_id.as_ref());
            poses.insert((x, y, type_s.clone(), owner_s.clone()), mobile_paint_pose_for(game, id, x, y, tick_fraction));
            mobile_map.entities.push(MapEntity {
                kind,
                owner: owner_s,
                type_id: type_s,
                health: 256,
                x,
                y,
                facing,
                sub_cell: 0,
                mission: Default::default(),
                tag: Default::default(),
            });
        }
        let lobby = self.lobby_primaries.clone();
        let mut composed = clean.clone();
        paint_mobiles_onto_preview_rgba(
            assets,
            &mobile_map,
            &mut composed,
            self.preview_origin.0,
            self.preview_origin.1,
            &mut self.paint,
            &|pal, owner| remap_owner_palette(rules, Some(&lobby), pal, owner),
            &|ent| poses.get(&(ent.x, ent.y, ent.type_id.clone(), ent.owner.clone())).copied().unwrap_or_default(),
        );
        for pending in &self.pending_buildups {
            let elapsed = pending.started.elapsed().as_millis() as u64;
            let frame = pending.clip.frame_at(elapsed).unwrap_or(0);
            paint_structure_buildup_onto_rgba(&mut composed, self.preview_origin.0, self.preview_origin.1, &pending.clip, frame);
        }
        for pending in &self.pending_teardowns {
            let Some(clip) = pending.clip.as_ref()
            else {
                continue;
            };
            let elapsed = pending.started.elapsed().as_millis() as u64;
            if let Some(frame) = clip.frame_at_reverse(elapsed) {
                paint_structure_buildup_onto_rgba(&mut composed, self.preview_origin.0, self.preview_origin.1, clip, frame);
            }
        }
        let clock_ms = self.anim_started.elapsed().as_millis() as u64;
        self.sync_preview_anim_lighting();
        paint_terrain_anims_onto_rgba(&mut composed, self.preview_origin.0, self.preview_origin.1, &self.terrain_anims, clock_ms);
        self.paint_ore_tree_frames_onto(&mut composed);
        paint_structure_anims_onto_rgba(&mut composed, self.preview_origin.0, self.preview_origin.1, &self.structure_anims, clock_ms);
        renderer.update_map_preview(composed);
        self.last_anim_sig = u64::MAX;
    }

    /// 上传当前 `preview_base`（可叠活动层与天气粒子）。定格后即使无 ActiveAnim 也必须调用。
    pub(super) fn present_preview_base(&mut self, renderer: &mut Renderer) {
        let Some(base) = self.preview_base.clone()
        else {
            return;
        };
        let mut composed = base;
        let clock_ms = self.anim_started.elapsed().as_millis() as u64;
        let has_anims = self.has_preview_anims();
        if has_anims {
            self.sync_preview_anim_lighting();
            paint_terrain_anims_onto_rgba(&mut composed, self.preview_origin.0, self.preview_origin.1, &self.terrain_anims, clock_ms);
            self.paint_ore_tree_frames_onto(&mut composed);
            paint_structure_anims_onto_rgba(&mut composed, self.preview_origin.0, self.preview_origin.1, &self.structure_anims, clock_ms);
            self.last_anim_sig = self.preview_anim_signature(clock_ms);
        }
        else {
            self.last_anim_sig = 0;
        }
        self.paint_weather_onto(&mut composed);
        renderer.update_map_preview(composed);
    }
}
