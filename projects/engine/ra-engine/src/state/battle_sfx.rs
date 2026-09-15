//! 玩法侧对局短音效队列（武器 `Report=` 等；壳层按 `sound.ini` 播）。

/// 一条应对局播放的短音效事件。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BattleSfxCue {
    /// `sound.ini` 事件 id（通常来自武器节 `Report=`）。
    pub event: String,
    /// 声源地图格；`None` 表示非空间音（UI / 无坐标路径），壳层按全音量播。
    pub cell: Option<(u16, u16)>,
}

impl crate::state::BattleState {
    /// 排队一条无坐标对局短音效（空事件忽略；同 tick 同事件只保留一条以免叠音）。
    pub fn push_battle_sfx_cue(&mut self, event: impl AsRef<str>) {
        self.push_battle_sfx_cue_at(event, None);
    }

    /// 排队一条对局短音效，可选声源格（供壳层按 `Range`/`MinVolume` 做距离衰减）。
    pub fn push_battle_sfx_cue_at(&mut self, event: impl AsRef<str>, cell: Option<(u16, u16)>) {
        let event = event.as_ref().trim();
        if event.is_empty() {
            return;
        }
        if let Some(existing) = self.pending_battle_sfx_cues.iter_mut().find(|c| c.event.eq_ignore_ascii_case(event)) {
            // 同事件已入队：若先前无坐标而本次有，补上坐标（不改事件去重口径）。
            if existing.cell.is_none() {
                existing.cell = cell;
            }
            return;
        }
        self.pending_battle_sfx_cues.push(BattleSfxCue { event: event.to_string(), cell });
    }

    /// 取出并清空本 tick 累计的对局短音效。
    pub fn take_battle_sfx_cues(&mut self) -> Vec<BattleSfxCue> {
        std::mem::take(&mut self.pending_battle_sfx_cues)
    }
}
