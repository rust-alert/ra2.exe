//! 玩法侧对局短音效队列（武器 `Report=` 等；壳层按 `sound.ini` 播）。

/// 一条应对局播放的短音效事件。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BattleSfxCue {
    /// `sound.ini` 事件 id（通常来自武器节 `Report=`）。
    pub event: String,
}

impl crate::state::BattleState {
    /// 排队一条对局短音效（空事件忽略；同 tick 同事件只保留一条以免叠音）。
    pub fn push_battle_sfx_cue(&mut self, event: impl AsRef<str>) {
        let event = event.as_ref().trim();
        if event.is_empty() {
            return;
        }
        if self.pending_battle_sfx_cues.iter().any(|c| c.event.eq_ignore_ascii_case(event)) {
            return;
        }
        self.pending_battle_sfx_cues.push(BattleSfxCue { event: event.to_string() });
    }

    /// 取出并清空本 tick 累计的对局短音效。
    pub fn take_battle_sfx_cues(&mut self) -> Vec<BattleSfxCue> {
        std::mem::take(&mut self.pending_battle_sfx_cues)
    }
}
