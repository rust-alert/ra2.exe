//! 壳层共享的会话：持有世界、转发命令、产出呈现快照。
//!
//! 不碰窗口与 GPU；可通过 `AssetSource` 装载遭遇战（见 `boot`）。

#![deny(missing_docs)]

mod boot;

use ra_map::{MapEntityKind, iso_to_screen, screen_to_iso};
use ra_net::{MatchFingerprint, StateDigest};
use ra_types::GameEdition;
use ra_world::{GameCommand, World};

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
    /// 当前选中实体下标（与 `units[].index` 对齐）。
    pub selected: Vec<usize>,
    /// 对局结束结果；未结束时为 `None`。
    pub outcome: Option<MatchOutcome>,
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
    /// 当前生命值。
    pub health: u32,
    /// 最大生命值。
    pub max_health: u32,
    /// 是否已死亡。
    pub dead: bool,
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
    /// 对局内容指纹（握手用；未设置时为空默认）。
    pub fingerprint: MatchFingerprint,
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
            fingerprint: MatchFingerprint { edition: String::new(), map: String::new(), rules_hash: 0 },
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
        self.world.advance_tick();
        self.selected.retain(|&i| i < self.world.entities.len() && !self.world.entities[i].dead);
        self.refresh_outcome();
    }

    /// 若仅剩一个阵营存活移动单位，锁定胜负并暂停。
    fn refresh_outcome(&mut self) {
        if self.outcome.is_some() {
            return;
        }
        let Some(owner) = self.sole_victor().map(str::to_string)
        else {
            return;
        };
        self.outcome = Some(MatchOutcome::Victory { owner: owner.clone() });
        self.paused = true;
        self.pause_reason = Some(format!("胜负已定 · {owner}"));
    }

    /// 单选一个存活移动单位。
    pub fn select_only(&mut self, index: usize) {
        self.selected.clear();
        if index < self.world.entities.len()
            && !self.world.entities[index].dead
            && matches!(
                self.world.entities[index].kind,
                MapEntityKind::Unit | MapEntityKind::Infantry | MapEntityKind::Aircraft
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
        if e.dead || !matches!(e.kind, MapEntityKind::Unit | MapEntityKind::Infantry | MapEntityKind::Aircraft) {
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

    /// 相对 `from` 最近的异阵营存活移动单位。
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
                    && matches!(e.kind, MapEntityKind::Unit | MapEntityKind::Infantry | MapEntityKind::Aircraft)
            })
            .min_by_key(|(_, e)| {
                let dx = i32::from(e.x) - i32::from(from.x);
                let dy = i32::from(e.y) - i32::from(from.y);
                dx * dx + dy * dy
            })
            .map(|(j, _)| j)
    }

    /// 若仅剩一个阵营仍有存活移动单位，返回其 owner。
    pub fn sole_victor(&self) -> Option<&str> {
        let mut owners: Vec<&str> = self
            .world
            .entities
            .iter()
            .filter(|e| !e.dead && matches!(e.kind, MapEntityKind::Unit | MapEntityKind::Infantry | MapEntityKind::Aircraft))
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
            .filter(|(_, e)| matches!(e.kind, MapEntityKind::Unit | MapEntityKind::Infantry | MapEntityKind::Aircraft))
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
                    health: e.health,
                    max_health: e.max_health,
                    dead: e.dead,
                }
            })
            .collect();
        RenderSnapshot {
            edition: self.world.edition,
            tick: self.world.tick,
            state_hash: self.world.state_hash(),
            units,
            selected: self.selected.clone(),
            outcome: self.outcome.clone(),
        }
    }
}
