//! 战斗会话生命周期：开局、暂停、推进与指纹。

use crate::{engine::EngineRuntime, game::commands::GameCommand, state::BattleState};
use ra_net::{MatchFingerprint, StateDigest};

use super::{
    outcome::{BattleOutcome, BattleStats},
    types::SessionBootKind,
};

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
    /// 开局契约（遭遇战 vs 战役）。
    pub boot_kind: SessionBootKind,
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
            boot_kind: SessionBootKind::Skirmish,
        }
    }

    /// 设置对局内容指纹（联机握手）。
    pub fn set_fingerprint(&mut self, fingerprint: MatchFingerprint) {
        self.fingerprint = fingerprint;
    }

    /// 写入装载时的对局随机种子。
    pub fn set_match_seed(&mut self, match_seed: u64) {
        self.match_seed = match_seed;
        self.world.match_seed = match_seed;
    }

    /// 由世界与装载备注打开一局遭遇战（设置预览原点与指纹）。
    pub fn open_skirmish(world: BattleState, boot_note: impl Into<String>, preview_origin: (i32, i32), fingerprint: MatchFingerprint) -> Self {
        let mut session = Self::new(world, boot_note);
        session.set_preview_origin(preview_origin.0, preview_origin.1);
        session.set_fingerprint(fingerprint);
        session.boot_kind = SessionBootKind::Skirmish;
        session.ai_enabled = true;
        session
    }

    /// 由世界与装载备注打开一局战役（保留预放单位；AI 默认关，剧本小队另行驱动）。
    pub fn open_campaign(world: BattleState, boot_note: impl Into<String>, preview_origin: (i32, i32), fingerprint: MatchFingerprint) -> Self {
        let mut session = Self::new(world, boot_note);
        session.set_preview_origin(preview_origin.0, preview_origin.1);
        session.set_fingerprint(fingerprint);
        session.boot_kind = SessionBootKind::Campaign;
        session.ai_enabled = false;
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
}
