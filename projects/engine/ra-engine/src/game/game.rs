//! 一局 RTS 游戏：权威状态、命令推进与结算。
//!
//! 不碰窗口与 GPU；不持有 UI 选中（选中属桌面 LocalPlayerController）。

use crate::{
    engine::EngineRuntime,
    game::{commands::GameCommand, reject::CommandReject},
    state::{
        BattleState,
        components::{AttackState, AnimationState, Health, Identity, MovementState, Owner, ProductionQueue, Transform},
    },
};
use ra_map::{MapEntityKind, iso_to_screen, screen_to_iso};
use ra_net::{MatchFingerprint, StateDigest};
use ra_types::{EntityId, GameEdition};

/// 默认仿真频率（与渲染帧率无关）。
pub const DEFAULT_TICK_HZ: u32 = 15;

/// 单次 `pump` 最多追赶的 tick 数，防止卡顿后螺旋追帧。
pub const MAX_TICKS_PER_PUMP: u32 = 8;

/// 一帧呈现用的不可变快照（渲染器应逐步只消费此类数据）。
#[derive(Debug, Clone)]
pub struct RenderSnapshot {
    /// 当前游戏版本。
    pub edition: GameEdition,
    /// 世界已推进的仿真 tick 数。
    pub tick: u64,
    /// 世界状态哈希（联机摘要用）。
    pub state_hash: u64,
    /// 可绘移动单位列表。
    pub units: Vec<SnapshotUnit>,
    /// 玩家经济与电力（供 HUD）。
    pub players: Vec<SnapshotPlayer>,
    /// 工厂生产队列与集结点。
    pub produce_queues: Vec<SnapshotProduceQueue>,
    /// 上一 tick 的命令拒绝（供错误反馈）。
    pub last_rejects: Vec<CommandReject>,
    /// 当前选中实体的稳定 ID（与 `units[].id` 对齐）。
    pub selected: Vec<EntityId>,
    /// 对局结束结果；未结束时为 `None`。
    pub outcome: Option<BattleOutcome>,
    /// 是否暂停（`pump` 不推进）。
    pub paused: bool,
    /// 暂停原因文案（胜负、手动暂停、摘要不一致等）。
    pub pause_reason: Option<String>,
    /// 结算统计；未结束时为 `None`。
    pub battle_stats: Option<BattleStats>,
    /// 当前会话画面（设置 / 对局中 / 结算）。
    pub screen: SessionScreen,
}

/// HUD / 标题栏用的轻量投影（不含单位表，避免第二遍全表扫描）。
#[derive(Debug, Clone)]
pub struct HudSnapshot {
    /// 世界已推进的仿真 tick 数。
    pub tick: u64,
    /// 玩家经济与电力。
    pub players: Vec<SnapshotPlayer>,
    /// 工厂生产队列。
    pub produce_queues: Vec<SnapshotProduceQueue>,
    /// 上一 tick 的命令拒绝。
    pub last_rejects: Vec<CommandReject>,
    /// 对局结束结果。
    pub outcome: Option<BattleOutcome>,
    /// 是否暂停。
    pub paused: bool,
    /// 暂停原因。
    pub pause_reason: Option<String>,
    /// 结算统计。
    pub battle_stats: Option<BattleStats>,
}

/// 会话画面（供桌面流程切换，不进入 BattleState tick）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SessionScreen {
    /// 对局进行中（含暂停）。
    InBattle,
    /// 结算画面。
    Results,
}

/// 快照中的玩家经济状态。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SnapshotPlayer {
    /// 阵营 / 房主名称。
    pub house: std::sync::Arc<str>,
    /// 当前资金。
    pub funds: i32,
    /// 供电量。
    pub power_output: i32,
    /// 耗电量。
    pub power_drain: i32,
    /// 是否低电。
    pub low_power: bool,
}

/// 快照中的一条工厂生产队列。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SnapshotProduceQueue {
    /// 工厂实体的稳定 ID。
    pub factory: EntityId,
    /// 正在生产的类型 ID。
    pub type_id: std::sync::Arc<str>,
    /// 剩余 tick。
    pub remaining_ticks: u32,
    /// 集结格 X。
    pub rally_x: Option<u16>,
    /// 集结格 Y。
    pub rally_y: Option<u16>,
}

/// 快照中的一个可绘实体。
#[derive(Debug, Clone)]
pub struct SnapshotUnit {
    /// 实体稳定 ID。
    pub id: EntityId,
    /// 实体种类（单位 / 步兵 / 飞行器）。
    pub kind: MapEntityKind,
    /// 规则中的类型 ID（与权威实体共享 `Arc`）。
    pub type_id: std::sync::Arc<str>,
    /// 所属阵营 owner（与权威实体共享 `Arc`）。
    pub owner: std::sync::Arc<str>,
    /// 地图格 X。
    pub x: u16,
    /// 地图格 Y。
    pub y: u16,
    /// 相对预览图画布的像素 X（已减 `preview_origin`）。
    pub screen_x: i32,
    /// 相对预览图画布的像素 Y（已减 `preview_origin`）。
    pub screen_y: i32,
    /// 车体朝向（0–255）。
    pub facing: u8,
    /// 炮塔朝向（0–255）。
    pub turret_facing: u8,
    /// 当前 HVA 动画帧。
    pub hva_frame: u16,
    /// 呈现用动画状态（由仿真快照派生，不推进 BattleState tick）。
    pub anim_state: AnimState,
    /// 当前生命值。
    pub health: u32,
    /// 最大生命值。
    pub max_health: u32,
    /// 是否已死亡。
    pub dead: bool,
    /// 是否可部署（如 MCV）；选中时呈现部署标记。
    pub deployable: bool,
    /// 移动最终目标相对预览图的锚点（无目标为 `None`；供选中行动线）。
    pub move_goal_screen: Option<(i32, i32)>,
    /// 当前攻击目标实体。
    pub attack_target: Option<EntityId>,
    /// 攻击目标相对预览图的锚点（供选中行动线终点）。
    pub attack_target_screen: Option<(i32, i32)>,
}

impl SnapshotUnit {
    /// 是否为建筑标记（相对菱形单位用方块绘制）。
    pub fn is_structure(&self) -> bool {
        matches!(self.kind, MapEntityKind::Structure)
    }
}

/// 单位/建筑呈现动画状态（A0 契约首批子集）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AnimState {
    /// 待机。
    Idle,
    /// 移动。
    Move,
    /// 攻击。
    Attack,
    /// 受击闪白。
    TakeDamage,
    /// 死亡。
    Die,
    /// 工厂生产中。
    Produce,
}

/// 对局结束结果（Alpha：唯一存活阵营胜）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BattleOutcome {
    /// 指定阵营获胜。
    Victory {
        /// 获胜阵营 owner 字符串。
        owner: String,
    },
}

/// 结算用统计（对局结束时锁定）。
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct BattleStats {
    /// 对局持续 tick。
    pub duration_ticks: u64,
    /// 已死亡移动单位数。
    pub units_lost: u32,
    /// 已死亡建筑数。
    pub buildings_lost: u32,
    /// 全场累计花费。
    pub funds_spent: i32,
}

/// 一场 RTS 权威战斗会话。
#[derive(Debug)]
pub struct BattleSession {
    /// 仿真世界（规则、地图、实体、通行格）。
    pub world: BattleState,
    /// 装载或启动时的备注（规则统计、实体数等）。
    pub boot_note: String,
    /// 预览图画布原点 X（等距屏幕坐标），用于点选逆变换。
    pub preview_origin_x: i32,
    /// 预览图画布原点 Y（等距屏幕坐标），用于点选逆变换。
    pub preview_origin_y: i32,
    /// 暂停时会话 `pump` 不推进。
    pub paused: bool,
    /// 暂停原因（如摘要不一致或胜负已定）。
    pub pause_reason: Option<String>,
    /// 对局结果；一旦设定则停止推进并拒绝新命令。
    pub outcome: Option<BattleOutcome>,
    /// 结算统计；对局结束时填充。
    pub battle_stats: Option<BattleStats>,
    /// 对局内容指纹（握手用；未设置时为空默认）。
    pub fingerprint: MatchFingerprint,
    /// 对局随机种子（装载时写入；混入指纹）。
    pub match_seed: u64,
    /// 是否为非本地阵营自动下发 AI 命令。
    pub ai_enabled: bool,
    /// 遭遇战难度标签（大厅选择；影响 AI 进攻/生产节奏）。
    pub difficulty: String,
}

impl BattleSession {
    /// 用已有世界与装载备注创建战斗会话（默认 tick 频率与空指纹）。
    pub fn new(world: BattleState, boot_note: impl Into<String>) -> Self {
        Self {
            world,
            boot_note: boot_note.into(),
            preview_origin_x: 0,
            preview_origin_y: 0,
            paused: false,
            pause_reason: None,
            outcome: None,
            battle_stats: None,
            fingerprint: MatchFingerprint { edition: String::new(), map: String::new(), rules_hash: 0 },
            match_seed: 0,
            ai_enabled: false,
            difficulty: "Normal".into(),
        }
    }

    /// 设置对局内容指纹（联机握手）。
    pub fn set_fingerprint(&mut self, fingerprint: MatchFingerprint) {
        self.fingerprint = fingerprint;
    }

    /// 写入装载时的对局随机种子。
    pub fn set_match_seed(&mut self, match_seed: u64) {
        self.match_seed = match_seed;
    }

    /// 由世界与装载备注打开一局（设置预览原点与指纹）。
    pub fn open_skirmish(world: BattleState, boot_note: impl Into<String>, preview_origin: (i32, i32), fingerprint: MatchFingerprint) -> Self {
        let mut session = Self::new(world, boot_note);
        session.set_preview_origin(preview_origin.0, preview_origin.1);
        session.set_fingerprint(fingerprint);
        session.ai_enabled = true;
        session
    }

    /// 写入遭遇战大厅所选难度（影响 AI 进攻与生产节奏）。
    pub fn set_difficulty(&mut self, difficulty: impl Into<String>) {
        self.difficulty = difficulty.into();
    }

    /// 构建对局指纹：规则字节 + 地图尺寸、实体数与随机种子混入。
    pub fn build_skirmish_fingerprint(
        edition: &str,
        map_name: &str,
        rules_bytes: &[u8],
        map_width: u32,
        map_height: u32,
        entity_count: usize,
        match_seed: u64,
    ) -> MatchFingerprint {
        let fp = MatchFingerprint::build(edition, map_name, rules_bytes);
        let mix = format!("{map_width}x{map_height}#{entity_count}#seed={match_seed:#x}");
        fp.mix_bytes(mix.as_bytes())
    }

    /// 清除暂停状态（胜负已定时无效）。
    pub fn resume(&mut self) {
        if self.outcome.is_some() {
            return;
        }
        self.paused = false;
        self.pause_reason = None;
    }

    /// 手动暂停（胜负已定时无效）。
    pub fn pause(&mut self, reason: impl Into<String>) {
        if self.outcome.is_some() {
            return;
        }
        self.paused = true;
        self.pause_reason = Some(reason.into());
    }

    /// 切换手动暂停；胜负已定时保持暂停。
    pub fn toggle_pause(&mut self) {
        if self.outcome.is_some() {
            return;
        }
        if self.paused {
            self.resume();
        }
        else {
            self.pause("已暂停");
        }
    }

    /// 本地状态摘要（联机上报用）。
    pub fn local_digest(&self) -> StateDigest {
        StateDigest { tick: self.world.tick, hash: self.world.state_hash() }
    }

    /// 与远端摘要比对。
    ///
    /// 仅在 **同 tick 且哈希相同** 时返回 `true`。tick 不一致或哈希不同均返回 `false`
    /// （tick 不一致不暂停，但不视为「已同步成功」）。
    pub fn apply_remote_digest(&mut self, remote: &StateDigest) -> bool {
        let local = self.local_digest();
        if remote.tick != local.tick {
            return false;
        }
        if remote.hash == local.hash {
            return true;
        }
        self.paused = true;
        self.pause_reason = Some(format!("摘要不一致 tick={} local={:#x} remote={:#x}", local.tick, local.hash, remote.hash));
        false
    }

    /// 设置预览图画布原点在等距屏幕空间中的偏移。
    pub fn set_preview_origin(&mut self, x: i32, y: i32) {
        self.preview_origin_x = x;
        self.preview_origin_y = y;
    }

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

    /// 向世界命令队列追加一条命令（对局已结束则忽略）。
    pub fn push_command(&mut self, cmd: GameCommand) {
        if self.outcome.is_some() {
            return;
        }
        self.world.push_command(cmd);
    }

    /// 推进恰好一个仿真 tick（由 `Session` 时钟驱动；阶段顺序来自 `runtime.schedule`）。
    pub fn advance_one_tick(&mut self, runtime: &EngineRuntime<'_>) {
        if self.ai_enabled {
            self.push_ai_commands();
        }
        self.world.advance_scheduled_tick(runtime.schedule);
        self.refresh_outcome();
    }

    /// 为所有非本地阵营下发本 tick 的 AI 命令（经 `push_command`）。
    ///
    /// `Easy`：奇数 tick 跳过生产与自动进攻，仅保留部署/建造节奏。
    /// `Normal`：每 4 个 tick 跳过一拍进攻/生产（略弱于 Hard）。
    /// `Hard`：每 tick 完整下发，并追加一轮生产尝试。
    fn push_ai_commands(&mut self) {
        let local_house = self.world.players.iter().find(|p| p.id == self.world.local_player).map(|p| p.house.clone());
        let opponents: Vec<(ra_types::PlayerId, std::sync::Arc<str>)> = self
            .world
            .players
            .iter()
            .filter(|p| local_house.as_ref().map(|h| p.house.as_ref() != h.as_ref()).unwrap_or(true))
            .map(|p| (p.id, p.house.clone()))
            .collect();
        let skip_offensive = difficulty_skips_offensive(&self.difficulty, self.world.tick);
        let hard_extra_produce = difficulty_extra_produce(&self.difficulty);
        for (player, house) in &opponents {
            let house = house.as_ref();
            let mut cmds = Vec::new();
            cmds.extend(crate::gameplay::ai::deploy_mcv_commands(&self.world, house));
            cmds.extend(crate::gameplay::ai::place_power_commands(&self.world, house, *player));
            cmds.extend(crate::gameplay::ai::place_barracks_commands(&self.world, house, *player));
            cmds.extend(crate::gameplay::ai::place_war_factory_commands(&self.world, house, *player));
            cmds.extend(crate::gameplay::ai::place_refinery_commands(&self.world, house, *player));
            if !skip_offensive {
                cmds.extend(crate::gameplay::ai::produce_infantry_commands(&self.world, house, *player));
                cmds.extend(crate::gameplay::ai::produce_vehicle_commands(&self.world, house, *player));
                cmds.extend(crate::gameplay::ai::auto_attack_commands(&self.world, house));
                if hard_extra_produce {
                    cmds.extend(crate::gameplay::ai::produce_infantry_commands(&self.world, house, *player));
                    cmds.extend(crate::gameplay::ai::produce_vehicle_commands(&self.world, house, *player));
                }
            }
            for cmd in cmds {
                self.world.push_player_command(*player, cmd);
            }
        }
    }

    /// 若仅剩一个阵营仍有作战力量，锁定胜负并暂停。
    fn refresh_outcome(&mut self) {
        if self.outcome.is_some() {
            return;
        }
        let Some(owner) = self.sole_victor().map(str::to_string)
        else {
            return;
        };
        self.battle_stats = Some(self.compute_battle_stats());
        self.outcome = Some(BattleOutcome::Victory { owner: owner.clone() });
        self.paused = true;
        self.pause_reason = Some(format!("胜负已定 · {owner}"));
    }

    fn compute_battle_stats(&self) -> BattleStats {
        let mut units_lost = 0u32;
        let mut buildings_lost = 0u32;
        for e in &self.world.entities {
            let id = e.id;
            if !self.world.ecs_get::<Health>(id).map(|h| h.dead).unwrap_or(false) {
                continue;
            }
            match self.world.ecs_get::<Identity>(id).map(|identity| identity.kind) {
                Some(MapEntityKind::Structure) => buildings_lost = buildings_lost.saturating_add(1),
                Some(MapEntityKind::Unit | MapEntityKind::Infantry | MapEntityKind::Aircraft) => {
                    units_lost = units_lost.saturating_add(1);
                }
                _ => {}
            }
        }
        let funds_spent = self.world.players.iter().map(|p| p.funds_spent).sum();
        BattleStats { duration_ticks: self.world.tick, units_lost, buildings_lost, funds_spent }
    }

    /// 指定实体移动到目标格。
    pub fn order_move(&mut self, selected: &[EntityId], x: u16, y: u16) {
        if self.outcome.is_some() {
            return;
        }
        for &id in selected {
            if self.world.entity_index(id).is_some() {
                self.push_command(GameCommand::MoveTo { entity: id, x, y });
            }
        }
    }

    /// 指定实体攻击目标。
    pub fn order_attack(&mut self, selected: &[EntityId], target: EntityId) {
        if self.outcome.is_some() {
            return;
        }
        if self.world.entity_index(target).is_none() {
            return;
        }
        for &id in selected {
            if id != target && self.world.entity_index(id).is_some() {
                self.push_command(GameCommand::Attack { attacker: id, target });
            }
        }
    }

    /// 部署指定可展开单位（如 MCV）。
    pub fn order_deploy(&mut self, selected: &[EntityId]) {
        if self.outcome.is_some() {
            return;
        }
        for &id in selected {
            if self.world.entity_index(id).is_some() {
                self.push_command(GameCommand::Deploy { entity: id });
            }
        }
    }

    /// 若实体可部署，返回目标建筑类型键（如 `GACNST` / `NACNST`）。
    pub fn deploy_target_of(&self, id: EntityId) -> Option<&str> {
        let (type_id, _) = self.world.ecs_identity(id)?;
        crate::gameplay::deploy_into_type(&self.world.definitions, type_id.as_ref())
    }

    /// 本地玩家在目标格放置建筑。
    pub fn order_place_building(&mut self, type_id: impl Into<String>, x: u16, y: u16) {
        if self.outcome.is_some() {
            return;
        }
        self.push_command(GameCommand::PlaceBuilding { player: self.world.local_player, type_id: type_id.into(), x, y });
    }

    /// 本地玩家排队生产单位。
    pub fn order_produce(&mut self, type_id: impl Into<String>) {
        if self.outcome.is_some() {
            return;
        }
        self.push_command(GameCommand::Produce { player: self.world.local_player, type_id: type_id.into() });
    }

    /// 为指定工厂设置集结点（非工厂由世界拒绝）。
    pub fn order_rally(&mut self, selected: &[EntityId], x: u16, y: u16) {
        if self.outcome.is_some() {
            return;
        }
        for &id in selected {
            if self.world.entity_index(id).is_some() {
                self.push_command(GameCommand::SetRallyPoint { factory: id, x, y });
            }
        }
    }

    /// 选中集合中是否包含建筑。
    pub fn selection_has_structure(&self, selected: &[EntityId]) -> bool {
        selected.iter().any(|&id| {
            !self.world.ecs_get::<Health>(id).map(|h| h.dead).unwrap_or(true)
                && self
                    .world
                    .ecs_get::<Identity>(id)
                    .map(|identity| identity.kind == MapEntityKind::Structure)
                    .unwrap_or(false)
        })
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

    /// 点选格上精确匹配的存活建筑。
    pub fn pick_structure_at(&self, x: u16, y: u16) -> Option<EntityId> {
        self.world.entities.iter().find_map(|e| {
            let id = e.id;
            if self.world.ecs_get::<Health>(id).map(|h| h.dead).unwrap_or(true) {
                return None;
            }
            if !self
                .world
                .ecs_get::<Identity>(id)
                .map(|identity| identity.kind == MapEntityKind::Structure)
                .unwrap_or(false)
            {
                return None;
            }
            let xf = self.world.ecs_get::<Transform>(id)?;
            (xf.x == x && xf.y == y).then_some(id)
        })
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
                if !matches!(
                    identity.kind,
                    MapEntityKind::Unit | MapEntityKind::Infantry | MapEntityKind::Aircraft | MapEntityKind::Structure
                ) {
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

    /// 若仅剩一个阵营仍有作战力量（存活建筑或可作战移动单位），返回其 owner。
    /// 至少需要两名玩家槽位，避免单机装载尚未开战时误判胜负。
    pub fn sole_victor(&self) -> Option<&str> {
        if self.world.players.len() < 2 {
            return None;
        }
        let mut owners: Vec<&str> = self
            .world
            .entities
            .iter()
            .filter_map(|e| {
                let id = e.id;
                if !is_combat_force(&self.world, id) {
                    return None;
                }
                self.world.ecs_get::<Owner>(id).map(|o| o.house.as_ref())
            })
            .collect();
        owners.sort_unstable();
        owners.dedup();
        if owners.len() == 1 { Some(owners[0]) } else { None }
    }

    /// 将指定实体投影为呈现用 `SnapshotUnit`（跳过非战斗可视种类）。
    ///
    /// 供脏集增量路径使用。不存在的 ID 被跳过。
    pub fn project_units(&self, ids: &[EntityId]) -> Vec<SnapshotUnit> {
        let mut out = Vec::with_capacity(ids.len());
        for &id in ids {
            if self.world.entity_index(id).is_none() {
                continue;
            }
            if !self
                .world
                .ecs_get::<Identity>(id)
                .map(|identity| {
                    matches!(
                        identity.kind,
                        MapEntityKind::Unit | MapEntityKind::Infantry | MapEntityKind::Aircraft | MapEntityKind::Structure
                    )
                })
                .unwrap_or(false)
            {
                continue;
            }
            if let Some(unit) = self.project_entity(id) {
                out.push(unit);
            }
        }
        out
    }

    fn project_entity(&self, id: EntityId) -> Option<SnapshotUnit> {
        let identity = self.world.ecs_get::<Identity>(id)?;
        let owner = self.world.ecs_get::<Owner>(id)?;
        let xf = self.world.ecs_get::<Transform>(id).copied()?;
        let health = self.world.ecs_get::<Health>(id).copied()?;
        let hva_frame = self.world.ecs_get::<AnimationState>(id).map(|a| a.hva_frame).unwrap_or(0);
        let z = self.world.pass_grid.cell_height(xf.x, xf.y);
        let (sx, sy) = iso_to_screen(i32::from(xf.x), i32::from(xf.y), z);
        let deployable = !matches!(identity.kind, MapEntityKind::Structure)
            && crate::gameplay::deploy_into_type(&self.world.definitions, identity.type_id.as_ref()).is_some();
        let movement = self.world.ecs_get::<MovementState>(id);
        let move_goal_screen = movement.and_then(|m| {
            let dx = m.destination_x?;
            let dy = m.destination_y?;
            Some(self.cell_anchor_screen(dx, dy))
        });
        let attack_target = self.world.ecs_get::<AttackState>(id).and_then(|a| a.target);
        let attack_target_screen = attack_target.and_then(|tid| {
            let txf = self.world.ecs_get::<Transform>(tid).copied()?;
            Some(self.cell_anchor_screen(txf.x, txf.y))
        });
        Some(SnapshotUnit {
            id,
            kind: identity.kind,
            type_id: identity.type_id.clone(),
            owner: owner.house.clone(),
            x: xf.x,
            y: xf.y,
            screen_x: sx - self.preview_origin_x,
            screen_y: sy - self.preview_origin_y,
            facing: xf.facing,
            turret_facing: xf.turret_facing,
            hva_frame,
            anim_state: derive_anim_state(&self.world, id),
            health: health.current,
            max_health: health.maximum,
            dead: health.dead,
            deployable,
            move_goal_screen,
            attack_target,
            attack_target_screen,
        })
    }

    /// 逻辑格 → 预览图锚点（与选中环中心同口径：`iso` 后再加菱形视觉偏移）。
    fn cell_anchor_screen(&self, x: u16, y: u16) -> (i32, i32) {
        let z = self.world.pass_grid.cell_height(x, y);
        let (sx, sy) = iso_to_screen(i32::from(x), i32::from(y), z);
        (sx - self.preview_origin_x + 30, sy - self.preview_origin_y + 15)
    }

    /// 从当前世界与本地选中构建一帧呈现快照。
    ///
    /// **原型路径**：每次全表扫描。单位投影经 [`Self::project_entity`]；
    /// 增量路径请优先使用脏集 + [`Self::project_units`]，HUD 用 [`Self::snapshot_hud`]。
    pub fn snapshot(&self, selected: &[EntityId]) -> RenderSnapshot {
        let units = self
            .world
            .entities
            .iter()
            .filter_map(|e| {
                let id = e.id;
                if !self
                    .world
                    .ecs_get::<Identity>(id)
                    .map(|identity| {
                        matches!(
                            identity.kind,
                            MapEntityKind::Unit | MapEntityKind::Infantry | MapEntityKind::Aircraft | MapEntityKind::Structure
                        )
                    })
                    .unwrap_or(false)
                {
                    return None;
                }
                self.project_entity(id)
            })
            .collect();
        let hud = self.snapshot_hud();
        RenderSnapshot {
            edition: self.world.edition,
            tick: hud.tick,
            state_hash: self.world.state_hash(),
            units,
            players: hud.players,
            produce_queues: hud.produce_queues,
            last_rejects: hud.last_rejects,
            selected: selected.to_vec(),
            outcome: hud.outcome,
            paused: hud.paused,
            pause_reason: hud.pause_reason,
            battle_stats: hud.battle_stats,
            screen: if self.outcome.is_some() { SessionScreen::Results } else { SessionScreen::InBattle },
        }
    }

    /// 轻量 HUD 投影：玩家、队列、拒绝与结算，不含单位表。
    pub fn snapshot_hud(&self) -> HudSnapshot {
        let players = self
            .world
            .players
            .iter()
            .map(|p| SnapshotPlayer {
                house: p.house.clone(),
                funds: p.funds,
                power_output: p.power_output,
                power_drain: p.power_drain,
                low_power: p.low_power(),
            })
            .collect();
        let produce_queues = self
            .world
            .entities
            .iter()
            .filter_map(|e| {
                let id = e.id;
                let queue = self.world.ecs_get::<ProductionQueue>(id)?;
                let (type_id, remaining_ticks) = queue.item.as_ref()?;
                Some(SnapshotProduceQueue {
                    factory: id,
                    type_id: type_id.clone(),
                    remaining_ticks: *remaining_ticks,
                    rally_x: queue.rally_x,
                    rally_y: queue.rally_y,
                })
            })
            .collect();
        HudSnapshot {
            tick: self.world.tick,
            players,
            produce_queues,
            last_rejects: self.world.last_rejects().to_vec(),
            outcome: self.outcome.clone(),
            paused: self.paused,
            pause_reason: self.pause_reason.clone(),
            battle_stats: self.battle_stats.clone(),
        }
    }
}

fn derive_anim_state(world: &BattleState, id: EntityId) -> AnimState {
    if world.ecs_get::<Health>(id).map(|h| h.dead).unwrap_or(true) {
        return AnimState::Die;
    }
    if world.ecs_get::<AnimationState>(id).map(|a| a.hit_flash > 0).unwrap_or(false) {
        return AnimState::TakeDamage;
    }
    if world.ecs_get::<ProductionQueue>(id).map(|p| p.item.is_some()).unwrap_or(false) {
        return AnimState::Produce;
    }
    if world.ecs_get::<AttackState>(id).map(|a| a.target.is_some()).unwrap_or(false) {
        return AnimState::Attack;
    }
    if world
        .ecs_get::<MovementState>(id)
        .map(|m| m.destination_x.is_some() || !m.path.is_empty())
        .unwrap_or(false)
    {
        return AnimState::Move;
    }
    AnimState::Idle
}

/// 按难度决定本 tick 是否跳过 AI 进攻/生产。
pub fn difficulty_skips_offensive(difficulty: &str, tick: u64) -> bool {
    if difficulty.eq_ignore_ascii_case("Easy") {
        tick % 2 == 1
    }
    else if difficulty.eq_ignore_ascii_case("Hard") {
        false
    }
    else {
        tick % 4 == 3
    }
}

/// Hard 是否追加一轮生产尝试。
pub fn difficulty_extra_produce(difficulty: &str) -> bool {
    difficulty.eq_ignore_ascii_case("Hard")
}

/// 冻结胜负：存活建筑或可作战移动单位均算作战力量。
fn is_combat_force(world: &BattleState, id: EntityId) -> bool {
    if world.ecs_get::<Health>(id).map(|h| h.dead).unwrap_or(true) {
        return false;
    }
    world
        .ecs_get::<Identity>(id)
        .map(|identity| {
            matches!(
                identity.kind,
                MapEntityKind::Unit | MapEntityKind::Infantry | MapEntityKind::Aircraft | MapEntityKind::Structure
            )
        })
        .unwrap_or(false)
}
