//! 一次运行会话：生命周期与时钟边界；内含可选的一局 Game。

use ra_types::BuiltinCapability;

use crate::{
    engine::EngineRuntime,
    game::{DEFAULT_TICK_HZ, Game, MAX_TICKS_PER_PUMP},
    state::MatchState,
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
    /// 对局进行中。
    Playing,
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
    /// 仿真频率（Hz）；由会话时钟驱动 `Game::advance_one_tick`。
    pub tick_hz: u32,
    /// 已累计、尚未消耗的毫秒（固定步长积分）。
    tick_accum_ms: f64,
    /// 当前一局游戏。
    game: Option<Game>,
}

impl Session {
    /// 空会话。
    pub fn new(spec: SessionSpec) -> Self {
        Self { spec, phase: SessionPhase::Idle, tick_hz: DEFAULT_TICK_HZ, tick_accum_ms: 0.0, game: None }
    }

    /// 测试 / 便利：由权威状态直接挂上一局 `Game` 并进入 Playing。
    pub fn from_state(state: MatchState, boot_note: impl Into<String>) -> Self {
        let note = boot_note.into();
        let mut session = Self::new(SessionSpec { label: note.clone(), required_capabilities: Vec::new() });
        session.attach_game(Game::new(state, note));
        session
    }

    /// 挂入已构造的一局游戏并进入 Playing。
    pub fn attach_game(&mut self, game: Game) {
        self.game = Some(game);
        self.phase = SessionPhase::Playing;
        self.tick_accum_ms = 0.0;
    }

    /// 当前游戏（只读）。
    pub fn game(&self) -> Option<&Game> {
        self.game.as_ref()
    }

    /// 当前游戏（可变）。
    pub fn game_mut(&mut self) -> Option<&mut Game> {
        self.game.as_mut()
    }

    /// 已挂载一局时返回引用（测试 / 调用方在 boot 后使用）。
    pub fn expect_game(&self) -> &Game {
        self.game.as_ref().expect("Session 尚无 Game：先 boot / attach_game / from_state")
    }

    /// 已挂载一局时返回可变引用。
    pub fn expect_game_mut(&mut self) -> &mut Game {
        self.game.as_mut().expect("Session 尚无 Game：先 boot / attach_game / from_state")
    }

    /// 取出游戏（结束 / 重开前）。
    pub fn take_game(&mut self) -> Option<Game> {
        self.phase = SessionPhase::Finished;
        self.game.take()
    }

    /// 强制推进恰好一个仿真 tick（测试 / 单步）。
    pub fn tick(&mut self, runtime: &EngineRuntime<'_>) {
        let Some(game) = self.game.as_mut()
        else {
            return;
        };
        if game.outcome.is_some() {
            return;
        }
        game.advance_one_tick(runtime);
    }

    /// 按真实时间推进 0..=`MAX_TICKS_PER_PUMP` 个仿真 tick。
    pub fn pump(&mut self, runtime: &EngineRuntime<'_>, dt_secs: f64) -> u32 {
        let Some(game) = self.game.as_mut()
        else {
            return 0;
        };
        if game.paused || game.outcome.is_some() || self.tick_hz == 0 {
            return 0;
        }
        let step_ms = 1000.0 / f64::from(self.tick_hz);
        self.tick_accum_ms += dt_secs.max(0.0) * 1000.0;
        let mut n = 0u32;
        while self.tick_accum_ms >= step_ms && n < MAX_TICKS_PER_PUMP {
            self.tick_accum_ms -= step_ms;
            game.advance_one_tick(runtime);
            n += 1;
            if game.outcome.is_some() {
                break;
            }
        }
        if self.tick_accum_ms > step_ms * f64::from(MAX_TICKS_PER_PUMP) {
            self.tick_accum_ms = 0.0;
        }
        n
    }
}
