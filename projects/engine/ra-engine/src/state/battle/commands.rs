use super::types::BattleState;
use crate::
game::{CommandReject, GameCommand, InputFrame};
use ra_types::{CommandId, PlayerId, ScheduledCommand, Tick};


impl BattleState {
    /// 入队命令载荷；自动包装为 [`ScheduledCommand`]（发出者为本地玩家，tick 为下一消费 tick）。
    pub fn push_command(&mut self, cmd: GameCommand) {
        self.push_player_command(self.local_player, cmd);
    }

    /// 以指定发出者入队命令载荷并包装调度信封。
    ///
    /// 信封 `player` 始终以本参数为准，**不得**被 `PlaceBuilding` / `Produce` 载荷内的 `player` 改写。
    /// 载荷与信封不一致时由 `apply_commands` 拒绝（`WrongOwner`）。
    pub fn push_player_command(&mut self, player: PlayerId, cmd: GameCommand) {
        let id = CommandId(self.next_command_id);
        self.next_command_id = self.next_command_id.saturating_add(1);
        let tick = Tick(self.tick.wrapping_add(1));
        self.pending_commands.push(ScheduledCommand::new(id, player, tick, cmd));
    }

    /// 直接入队已调度命令（测试 / 网络回放）。
    pub fn push_scheduled(&mut self, cmd: ScheduledCommand) {
        self.pending_commands.push(cmd);
    }

    /// 上一 tick 的输入帧（无操作时也为空命令列表）。
    pub fn last_input_frame(&self) -> &InputFrame {
        &self.last_input_frame
    }

    /// 上一 tick 产生的命令拒绝记录。
    pub fn last_rejects(&self) -> &[CommandReject] {
        &self.last_rejects
    }
}
