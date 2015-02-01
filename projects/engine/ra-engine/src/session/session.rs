//! 一次运行会话：生命周期与时钟边界；内含可选的一场 `BattleSession`。

use ra_types::BuiltinCapability;

use crate::{
    engine::EngineRuntime,
    game::{DEFAULT_TICK_HZ, BattleSession, MAX_TICKS_PER_PUMP},
    state::BattleState,
};

/// 创建会话时的规格。
///
/// `required_capabilities` 须全部出现在引擎能力表中，否则 `validate_session_spec` 失败。
#[derive(Debug, Clone, Default)]
pub struct SessionSpec {
    /// 可选备注（长度有上限，防止无界标签）。
    pub label: String,
    /// 创建会话前必须已在引擎定义中声明的内置能力。空表示不额外要求。
    pub required_capabilities: Vec<BuiltinCapability>,
}

/// 会话阶段。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum SessionPhase {
    /// 尚未开始一局。
    #[default]
    Idle,
    /// 战斗进行中。
    InBattle,
    /// 已结束（可重开）。
    Finished,
}

/// 一次运行会话（桌面单机、回放、测试等）。
#[derive(Debug)]
pub struct Session {
    /// 规格。
    pub spec: SessionSpec,
    /// 阶段。
    pub phase: SessionPhase,
    /// 仿真频率（Hz）；由会话时钟驱动 `BattleSession::advance_one_tick`。
    pub tick_hz: u32,
    /// 已累计、尚未消耗的毫秒（固定步长积分）。
    tick_accum_ms: f64,
    /// 当前一场战斗。
    battle: Option<BattleSession>,
}

impl Session {
    /// 空会话。
    pub fn new(spec: SessionSpec) -> Self {
        Self { spec, phase: SessionPhase::Idle, tick_hz: DEFAULT_TICK_HZ, tick_accum_ms: 0.0, battle: None }
    }

    /// 测试 / 便利：由权威状态直接挂上一场 `BattleSession` 并进入 `InBattle`。
    pub fn from_state(state: BattleState, boot_note: impl Into<String>) -> Self {
        let note = boot_note.into();
        let mut session = Self::new(SessionSpec { label: note.clone(), required_capabilities: Vec::new() });
        session.attach_battle(BattleSession::new(state, note));
        session
    }

    /// 挂入已构造的一场战斗并进入 `InBattle`。
    pub fn attach_battle(&mut self, battle: BattleSession) {
        self.battle = Some(battle);
        self.phase = SessionPhase::InBattle;
        self.tick_accum_ms = 0.0;
    }

    /// 当前战斗会话（只读）。
    pub fn battle(&self) -> Option<&BattleSession> {
        self.battle.as_ref()
    }

    /// 当前战斗会话（可变）。
    pub fn battle_mut(&mut self) -> Option<&mut BattleSession> {
        self.battle.as_mut()
    }

    /// 已挂载一局时返回引用（测试 / 调用方在 boot 后使用）。
    pub fn expect_battle(&self) -> &BattleSession {
        self.battle.as_ref().expect("Session 尚无 BattleSession：先 boot / attach_battle / from_state")
    }

    /// 已挂载一局时返回可变引用。
    pub fn expect_battle_mut(&mut self) -> &mut BattleSession {
        self.battle.as_mut().expect("Session 尚无 BattleSession：先 boot / attach_battle / from_state")
    }

    /// 取出战斗会话（结束 / 重开前）。
    pub fn take_battle(&mut self) -> Option<BattleSession> {
        self.phase = SessionPhase::Finished;
        self.battle.take()
    }

    /// 强制推进恰好一个仿真 tick（测试 / 单步）。
    pub fn tick(&mut self, runtime: &EngineRuntime<'_>) {
        let Some(battle) = self.battle.as_mut() else {
            return;
        };
        if battle.outcome.is_some() {
            return;
        }
        battle.advance_one_tick(runtime);
    }

    /// 按真实时间推进 0..=`MAX_TICKS_PER_PUMP` 个仿真 tick。
    pub fn pump(&mut self, runtime: &EngineRuntime<'_>, dt_secs: f64) -> u32 {
        let Some(battle) = self.battle.as_mut() else {
            return 0;
        };
        if battle.paused || battle.outcome.is_some() || self.tick_hz == 0 {
            return 0;
        }
        let step_ms = 1000.0 / f64::from(self.tick_hz);
        self.tick_accum_ms += dt_secs.max(0.0) * 1000.0;
        let mut n = 0u32;
        while self.tick_accum_ms >= step_ms && n < MAX_TICKS_PER_PUMP {
            self.tick_accum_ms -= step_ms;
            battle.advance_one_tick(runtime);
            n += 1;
            if battle.outcome.is_some() {
                break;
            }
        }
        if self.tick_accum_ms > step_ms * f64::from(MAX_TICKS_PER_PUMP) {
            self.tick_accum_ms = 0.0;
        }
        n
    }
}
