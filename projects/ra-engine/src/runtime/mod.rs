//! 对局运行时：持有世界、转发命令、产出呈现快照。
//!
//! 不碰窗口与 GPU；可通过 `AssetSource` 装载遭遇战（见 `boot`）。

mod ai;
mod boot;

use ra_map::{MapEntityKind, iso_to_screen, screen_to_iso};
use ra_net::{MatchFingerprint, StateDigest};
use ra_types::GameEdition;
use crate::{CommandReject, GameCommand, World};

pub use boot::{SkirmishOpenResult, open_skirmish_session};

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
    /// 当前选中实体下标（与 `units[].index` 对齐）。
    pub selected: Vec<usize>,
    /// 对局结束结果；未结束时为 `None`。
    pub outcome: Option<MatchOutcome>,
    /// 是否暂停（`pump` 不推进）。
    pub paused: bool,
    /// 暂停原因文案（胜负、手动暂停、摘要不一致等）。
    pub pause_reason: Option<String>,
    /// 结算统计；未结束时为 `None`。
    pub match_stats: Option<MatchStats>,
    /// 当前会话画面（设置 / 对局中 / 结算）。
    pub screen: SessionScreen,
}

/// 会话画面（供桌面流程切换，不进入 World tick）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SessionScreen {
    /// 对局进行中（含暂停）。
    InMatch,
    /// 结算画面。
    Results,
}

/// 快照中的玩家经济状态。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SnapshotPlayer {
    /// 阵营 / 房主名称。
    pub house: String,
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
    /// 工厂在 `World::entities` 中的下标。
    pub factory_index: usize,
    /// 正在生产的类型 ID。
    pub type_id: String,
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
    /// 在 `World::entities` 中的下标。
    pub index: usize,
    /// 实体种类（单位 / 步兵 / 飞行器）。
    pub kind: MapEntityKind,
    /// 规则中的类型 ID。
    pub type_id: String,
    /// 所属阵营 owner 字符串。
    pub owner: String,
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
    /// 呈现用动画状态（由仿真快照派生，不推进 World tick）。
    pub anim_state: AnimState,
    /// 当前生命值。
    pub health: u32,
    /// 最大生命值。
    pub max_health: u32,
    /// 是否已死亡。
    pub dead: bool,
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
pub enum MatchOutcome {
    /// 指定阵营获胜。
    Victory {
        /// 获胜阵营 owner 字符串。
        owner: String,
    },
}

/// 结算用统计（对局结束时锁定）。
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct MatchStats {
    /// 对局持续 tick。
    pub duration_ticks: u64,
    /// 已死亡移动单位数。
    pub units_lost: u32,
    /// 已死亡建筑数。
    pub buildings_lost: u32,
    /// 全场累计花费。
    pub funds_spent: i32,
}

/// 运行中会话。
#[derive(Debug)]
pub struct Session {
    /// 仿真世界（规则、地图、实体、通行格）。
    pub world: World,
    /// 装载或启动时的备注（规则统计、实体数等）。
    pub boot_note: String,
    /// 当前选中的实体下标（本地玩家操作）。
    pub selected: Vec<usize>,
    /// 预览图画布原点 X（等距屏幕坐标），用于点选逆变换。
    pub preview_origin_x: i32,
    /// 预览图画布原点 Y（等距屏幕坐标），用于点选逆变换。
    pub preview_origin_y: i32,
    /// 仿真频率（Hz）。
    pub tick_hz: u32,
    /// 已累计、尚未消耗的毫秒（固定步长积分）。
    tick_accum_ms: f64,
    /// 暂停时 `pump` 不推进。
    pub paused: bool,
    /// 暂停原因（如摘要不一致或胜负已定）。
    pub pause_reason: Option<String>,
    /// 对局结果；一旦设定则停止推进并拒绝新命令。
    pub outcome: Option<MatchOutcome>,
    /// 结算统计；对局结束时填充。
    pub match_stats: Option<MatchStats>,
    /// 对局内容指纹（握手用；未设置时为空默认）。
    pub fingerprint: MatchFingerprint,
    /// 是否为非本地阵营自动下发 AI 命令。
    pub ai_enabled: bool,
}

impl Session {
    /// 用已有世界与装载备注创建会话（默认 tick 频率与空指纹）。
    pub fn new(world: World, boot_note: impl Into<String>) -> Self {
        Self {
            world,
            boot_note: boot_note.into(),
            selected: Vec::new(),
            preview_origin_x: 0,
            preview_origin_y: 0,
            tick_hz: DEFAULT_TICK_HZ,
            tick_accum_ms: 0.0,
            paused: false,
            pause_reason: None,
            outcome: None,
            match_stats: None,
            fingerprint: MatchFingerprint { edition: String::new(), map: String::new(), rules_hash: 0 },
            ai_enabled: false,
        }
    }

    /// 设置对局内容指纹（联机握手）。
    pub fn set_fingerprint(&mut self, fingerprint: MatchFingerprint) {
        self.fingerprint = fingerprint;
    }

    /// 由世界与装载备注打开一局（设置预览原点与指纹）。
    pub fn open_skirmish(
        world: World,
        boot_note: impl Into<String>,
        preview_origin: (i32, i32),
        fingerprint: MatchFingerprint,
    ) -> Self {
        let mut session = Self::new(world, boot_note);
        session.set_preview_origin(preview_origin.0, preview_origin.1);
        session.set_fingerprint(fingerprint);
        session.ai_enabled = true;
        session
    }

    /// 构建对局指纹：规则字节 + 地图尺寸与实体数混入。
    pub fn build_skirmish_fingerprint(
        edition: &str,
        map_name: &str,
        rules_bytes: &[u8],
        map_width: u32,
        map_height: u32,
        entity_count: usize,
    ) -> MatchFingerprint {
        let fp = MatchFingerprint::build(edition, map_name, rules_bytes);
        let mix = format!("{map_width}x{map_height}#{entity_count}");
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

    /// 与远端摘要比对；同 tick 且哈希不同则暂停。返回是否一致（或暂不可比）。
    pub fn apply_remote_digest(&mut self, remote: &StateDigest) -> bool {
        let local = self.local_digest();
        if remote.tick != local.tick {
            return true;
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
    pub fn pick_mobile_at(&self, x: u16, y: u16) -> Option<usize> {
        let mut best: Option<(u32, usize)> = None;
        for (i, e) in self.world.entities.iter().enumerate() {
            if e.dead || !matches!(e.kind, MapEntityKind::Unit | MapEntityKind::Infantry | MapEntityKind::Aircraft) {
                continue;
            }
            let dist = (i32::from(e.x) - i32::from(x)).unsigned_abs() + (i32::from(e.y) - i32::from(y)).unsigned_abs();
            if dist > 1 {
                continue;
            }
            if best.map(|(d, _)| dist < d).unwrap_or(true) {
                best = Some((dist, i));
            }
        }
        best.map(|(_, i)| i)
    }

    /// 向世界命令队列追加一条命令（对局已结束则忽略）。
    pub fn push_command(&mut self, cmd: GameCommand) {
        if self.outcome.is_some() {
            return;
        }
        self.world.push_command(cmd);
    }

    /// 强制推进恰好一个仿真 tick（测试 / 单步）。
    pub fn tick(&mut self) {
        if self.outcome.is_some() {
            return;
        }
        self.advance_one_tick();
    }

    /// 按真实时间推进 0..=`MAX_TICKS_PER_PUMP` 个仿真 tick。
    pub fn pump(&mut self, dt_secs: f64) -> u32 {
        if self.paused || self.outcome.is_some() || self.tick_hz == 0 {
            return 0;
        }
        let step_ms = 1000.0 / f64::from(self.tick_hz);
        self.tick_accum_ms += dt_secs.max(0.0) * 1000.0;
        let mut n = 0u32;
        while self.tick_accum_ms >= step_ms && n < MAX_TICKS_PER_PUMP {
            self.tick_accum_ms -= step_ms;
            self.advance_one_tick();
            n += 1;
            if self.outcome.is_some() {
                break;
            }
        }
        if self.tick_accum_ms > step_ms * f64::from(MAX_TICKS_PER_PUMP) {
            self.tick_accum_ms = 0.0;
        }
        n
    }

    fn advance_one_tick(&mut self) {
        if self.ai_enabled {
            self.push_ai_commands();
        }
        self.world.advance_tick();
        self.selected.retain(|&i| i < self.world.entities.len() && !self.world.entities[i].dead);
        self.refresh_outcome();
    }

    /// 为所有非本地阵营下发本 tick 的 AI 命令（经 `push_command`）。
    fn push_ai_commands(&mut self) {
        let local_house = self
            .world
            .players
            .iter()
            .find(|p| p.id == self.world.local_player)
            .map(|p| p.house.clone());
        let opponents: Vec<(ra_types::PlayerId, String)> = self
            .world
            .players
            .iter()
            .filter(|p| local_house.as_ref().map(|h| &p.house != h).unwrap_or(true))
            .map(|p| (p.id, p.house.clone()))
            .collect();
        let mut cmds = Vec::new();
        for (player, house) in &opponents {
            cmds.extend(ai::deploy_mcv_commands(&self.world, house));
            cmds.extend(ai::place_power_commands(&self.world, house, *player));
            cmds.extend(ai::place_barracks_commands(&self.world, house, *player));
            cmds.extend(ai::place_war_factory_commands(&self.world, house, *player));
            cmds.extend(ai::place_refinery_commands(&self.world, house, *player));
            cmds.extend(ai::produce_infantry_commands(&self.world, house, *player));
            cmds.extend(ai::produce_vehicle_commands(&self.world, house, *player));
            cmds.extend(ai::auto_attack_commands(&self.world, house));
        }
        for cmd in cmds {
            self.push_command(cmd);
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
        self.match_stats = Some(self.compute_match_stats());
        self.outcome = Some(MatchOutcome::Victory { owner: owner.clone() });
        self.paused = true;
        self.pause_reason = Some(format!("胜负已定 · {owner}"));
    }

    fn compute_match_stats(&self) -> MatchStats {
        let mut units_lost = 0u32;
        let mut buildings_lost = 0u32;
        for e in &self.world.entities {
            if !e.dead {
                continue;
            }
            match e.kind {
                MapEntityKind::Structure => buildings_lost = buildings_lost.saturating_add(1),
                MapEntityKind::Unit | MapEntityKind::Infantry | MapEntityKind::Aircraft => {
                    units_lost = units_lost.saturating_add(1);
                }
            }
        }
        let funds_spent = self.world.players.iter().map(|p| p.funds_spent).sum();
        MatchStats {
            duration_ticks: self.world.tick,
            units_lost,
            buildings_lost,
            funds_spent,
        }
    }

    /// 单选一个存活实体（单位或建筑）。
    pub fn select_only(&mut self, index: usize) {
        self.selected.clear();
        if index < self.world.entities.len()
            && !self.world.entities[index].dead
            && matches!(
                self.world.entities[index].kind,
                MapEntityKind::Unit
                    | MapEntityKind::Infantry
                    | MapEntityKind::Aircraft
                    | MapEntityKind::Structure
            )
        {
            self.selected.push(index);
        }
    }

    /// 若可多选则加入选中（已在选中则忽略）；与已选不同阵营则拒绝。
    pub fn select_add(&mut self, index: usize) {
        if index >= self.world.entities.len() {
            return;
        }
        let e = &self.world.entities[index];
        if e.dead
            || !matches!(
                e.kind,
                MapEntityKind::Unit
                    | MapEntityKind::Infantry
                    | MapEntityKind::Aircraft
                    | MapEntityKind::Structure
            )
        {
            return;
        }
        if let Some(&first) = self.selected.first() {
            if self.world.entities[first].owner != e.owner {
                return;
            }
        }
        if !self.selected.contains(&index) {
            self.selected.push(index);
        }
    }

    /// 选中与 `index` 同阵营的全部存活移动单位。
    pub fn select_all_of_owner(&mut self, index: usize) {
        if index >= self.world.entities.len() {
            return;
        }
        let owner = self.world.entities[index].owner.clone();
        self.selected.clear();
        for (i, e) in self.world.entities.iter().enumerate() {
            if e.dead || e.owner != owner {
                continue;
            }
            if matches!(e.kind, MapEntityKind::Unit | MapEntityKind::Infantry | MapEntityKind::Aircraft) {
                self.selected.push(i);
            }
        }
    }

    /// 在存活移动单位间循环选中。
    pub fn cycle_selection(&mut self) {
        let mobiles: Vec<usize> = self
            .world
            .entities
            .iter()
            .enumerate()
            .filter(|(_, e)| {
                !e.dead && matches!(e.kind, MapEntityKind::Unit | MapEntityKind::Infantry | MapEntityKind::Aircraft)
            })
            .map(|(i, _)| i)
            .collect();
        if mobiles.is_empty() {
            self.selected.clear();
            return;
        }
        let next = match self.selected.first() {
            Some(&cur) => {
                mobiles.iter().position(|&i| i == cur).map(|p| mobiles[(p + 1) % mobiles.len()]).unwrap_or(mobiles[0])
            }
            None => mobiles[0],
        };
        self.select_only(next);
    }

    /// 选中单位移动到目标格。
    pub fn order_selected_move(&mut self, x: u16, y: u16) {
        if self.outcome.is_some() {
            return;
        }
        for &i in &self.selected.clone() {
            self.push_command(GameCommand::MoveTo { entity_index: i, x, y });
        }
    }

    /// 选中单位攻击目标。
    pub fn order_selected_attack(&mut self, target_index: usize) {
        if self.outcome.is_some() {
            return;
        }
        for &i in &self.selected.clone() {
            if i != target_index {
                self.push_command(GameCommand::Attack { attacker_index: i, target_index });
            }
        }
    }

    /// 部署当前选中的可展开单位（如 MCV）。
    pub fn order_selected_deploy(&mut self) {
        if self.outcome.is_some() {
            return;
        }
        for &entity_index in &self.selected.clone() {
            self.push_command(GameCommand::Deploy { entity_index });
        }
    }

    /// 本地玩家在目标格放置建筑。
    pub fn order_place_building(&mut self, type_id: impl Into<String>, x: u16, y: u16) {
        if self.outcome.is_some() {
            return;
        }
        self.push_command(GameCommand::PlaceBuilding {
            player: self.world.local_player,
            type_id: type_id.into(),
            x,
            y,
        });
    }

    /// 本地玩家排队生产单位。
    pub fn order_produce(&mut self, type_id: impl Into<String>) {
        if self.outcome.is_some() {
            return;
        }
        self.push_command(GameCommand::Produce {
            player: self.world.local_player,
            type_id: type_id.into(),
        });
    }

    /// 为当前选中的工厂设置集结点（非工厂由世界拒绝）。
    pub fn order_selected_rally(&mut self, x: u16, y: u16) {
        if self.outcome.is_some() {
            return;
        }
        for &factory_index in &self.selected.clone() {
            self.push_command(GameCommand::SetRallyPoint { factory_index, x, y });
        }
    }

    /// 选中集合中是否包含建筑。
    pub fn selection_has_structure(&self) -> bool {
        self.selected.iter().any(|&i| {
            self.world.entities.get(i).is_some_and(|e| !e.dead && e.kind == MapEntityKind::Structure)
        })
    }

    /// 点选格上或其四邻的存活实体（单位优先，其次建筑）。
    pub fn pick_entity_at(&self, x: u16, y: u16) -> Option<usize> {
        self.pick_mobile_at(x, y).or_else(|| self.pick_structure_at(x, y))
    }

    /// 点选格上精确匹配的存活建筑。
    pub fn pick_structure_at(&self, x: u16, y: u16) -> Option<usize> {
        self.world.entities.iter().position(|e| {
            !e.dead && e.kind == MapEntityKind::Structure && e.x == x && e.y == y
        })
    }

    /// 相对 `from` 最近的异阵营存活目标（移动单位或建筑）。
    pub fn nearest_hostile(&self, from_index: usize) -> Option<usize> {
        let from = self.world.entities.get(from_index)?;
        if from.dead {
            return None;
        }
        self.world
            .entities
            .iter()
            .enumerate()
            .filter(|(j, e)| {
                *j != from_index
                    && !e.dead
                    && e.owner != from.owner
                    && matches!(
                        e.kind,
                        MapEntityKind::Unit
                            | MapEntityKind::Infantry
                            | MapEntityKind::Aircraft
                            | MapEntityKind::Structure
                    )
            })
            .min_by_key(|(_, e)| {
                let dx = i32::from(e.x) - i32::from(from.x);
                let dy = i32::from(e.y) - i32::from(from.y);
                dx * dx + dy * dy
            })
            .map(|(j, _)| j)
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
            .filter(|e| is_combat_force(e))
            .map(|e| e.owner.as_str())
            .collect();
        owners.sort_unstable();
        owners.dedup();
        if owners.len() == 1 { Some(owners[0]) } else { None }
    }

    /// 从当前世界与选中状态构建一帧呈现快照。
    pub fn snapshot(&self) -> RenderSnapshot {
        let units = self
            .world
            .entities
            .iter()
            .enumerate()
            .filter(|(_, e)| {
                matches!(
                    e.kind,
                    MapEntityKind::Unit | MapEntityKind::Infantry | MapEntityKind::Aircraft | MapEntityKind::Structure
                )
            })
            .map(|(index, e)| {
                let z = self.world.pass_grid.cell_height(e.x, e.y);
                let (sx, sy) = iso_to_screen(i32::from(e.x), i32::from(e.y), z);
                SnapshotUnit {
                    index,
                    kind: e.kind,
                    type_id: e.type_id.clone(),
                    owner: e.owner.clone(),
                    x: e.x,
                    y: e.y,
                    screen_x: sx - self.preview_origin_x,
                    screen_y: sy - self.preview_origin_y,
                    facing: e.facing,
                    turret_facing: e.turret_facing,
                    hva_frame: e.hva_frame,
                    anim_state: derive_anim_state(e),
                    health: e.health,
                    max_health: e.max_health,
                    dead: e.dead,
                }
            })
            .collect();
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
            .enumerate()
            .filter_map(|(factory_index, e)| {
                let (type_id, remaining_ticks) = e.produce_queue.as_ref()?;
                Some(SnapshotProduceQueue {
                    factory_index,
                    type_id: type_id.clone(),
                    remaining_ticks: *remaining_ticks,
                    rally_x: e.rally_x,
                    rally_y: e.rally_y,
                })
            })
            .collect();
        RenderSnapshot {
            edition: self.world.edition,
            tick: self.world.tick,
            state_hash: self.world.state_hash(),
            units,
            players,
            produce_queues,
            last_rejects: self.world.last_rejects().to_vec(),
            selected: self.selected.clone(),
            outcome: self.outcome.clone(),
            paused: self.paused,
            pause_reason: self.pause_reason.clone(),
            match_stats: self.match_stats.clone(),
            screen: if self.outcome.is_some() {
                SessionScreen::Results
            } else {
                SessionScreen::InMatch
            },
        }
    }
}

fn derive_anim_state(e: &crate::WorldEntity) -> AnimState {
    if e.dead {
        return AnimState::Die;
    }
    if e.hit_flash > 0 {
        return AnimState::TakeDamage;
    }
    if e.produce_queue.is_some() {
        return AnimState::Produce;
    }
    if e.attack_target.is_some() {
        return AnimState::Attack;
    }
    if e.target_x.is_some() || !e.path.is_empty() {
        return AnimState::Move;
    }
    AnimState::Idle
}

/// 冻结胜负：存活建筑或可作战移动单位均算作战力量。
fn is_combat_force(e: &crate::WorldEntity) -> bool {
    !e.dead
        && matches!(
            e.kind,
            MapEntityKind::Unit | MapEntityKind::Infantry | MapEntityKind::Aircraft | MapEntityKind::Structure
        )
}
