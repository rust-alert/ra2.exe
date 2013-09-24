//! 一次运行会话：生命周期与时钟边界；内含可选的一局 Game。

use crate::game::Game;
use crate::state::MatchState;

/// 创建会话时的规格（骨架）。
#[derive(Debug, Clone, Default)]
pub struct SessionSpec {
    /// 可选备注。
    pub label: String,
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
    /// 当前一局游戏。
    game: Option<Game>,
}

impl Session {
    /// 空会话。
    pub fn new(spec: SessionSpec) -> Self {
        Self {
            spec,
            phase: SessionPhase::Idle,
            game: None,
        }
    }

    /// 测试 / 便利：由权威状态直接挂上一局 `Game` 并进入 Playing。
    pub fn from_state(state: MatchState, boot_note: impl Into<String>) -> Self {
        let note = boot_note.into();
        let mut session = Self::new(SessionSpec {
            label: note.clone(),
        });
        session.attach_game(Game::new(state, note));
        session
    }

    /// 挂入已构造的一局游戏并进入 Playing。
    pub fn attach_game(&mut self, game: Game) {
        self.game = Some(game);
        self.phase = SessionPhase::Playing;
    }

    /// 当前游戏（只读）。
    pub fn game(&self) -> Option<&Game> {
        self.game.as_ref()
    }

    /// 当前游戏（可变）。
    pub fn game_mut(&mut self) -> Option<&mut Game> {
        self.game.as_mut()
    }

    /// 取出游戏（结束 / 重开前）。
    pub fn take_game(&mut self) -> Option<Game> {
        self.phase = SessionPhase::Finished;
        self.game.take()
    }
}

impl std::ops::Deref for Session {
    type Target = Game;

    fn deref(&self) -> &Self::Target {
        self.game.as_ref().expect("Session 尚无 Game：先 boot / attach_game / start")
    }
}

impl std::ops::DerefMut for Session {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.game.as_mut().expect("Session 尚无 Game：先 boot / attach_game / start")
    }
}
