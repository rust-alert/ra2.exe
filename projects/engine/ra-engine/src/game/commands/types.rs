use ra_types::{CommandBody, ScheduledCommand};

/// 可执行命令载荷（跨层与 `ra_types::CommandBody` 共用）。
pub type GameCommand = CommandBody;

/// 一个仿真 tick 的完整输入帧。
///
/// 联机锁步要求：**每个 tick 都必须有一帧**；本 tick 无操作时 `commands` 为空。
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct InputFrame {
    /// 本帧对应的逻辑 tick。
    pub tick: u64,
    /// 本 tick 消费的已调度命令（可为空）。
    pub commands: Vec<ScheduledCommand>,
}

impl InputFrame {
    /// 构造仅含 tick、无命令的空输入帧。
    pub fn empty(tick: u64) -> Self {
        Self { tick, commands: Vec::new() }
    }

    /// 本帧是否没有任何命令。
    pub fn is_empty(&self) -> bool {
        self.commands.is_empty()
    }
}
