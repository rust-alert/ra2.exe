//! 玩法侧 EVA 提示队列（按 house 定向，壳层只播本机阵营）。

use std::sync::Arc;

/// 一条应对某阵营播放的 EVA 事件。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EvaCue {
    /// 应收听的阵营 house 名。
    pub house: Arc<str>,
    /// `eva.ini` / `evamd.ini` 事件 id（如 `EVA_UnitReady`）。
    pub event: &'static str,
}

impl crate::state::BattleState {
    /// 向指定阵营排队一条 EVA（空 house / 空事件忽略）。
    pub fn push_eva_cue(&mut self, house: impl AsRef<str>, event: &'static str) {
        let house = house.as_ref().trim();
        if house.is_empty() || event.is_empty() {
            return;
        }
        self.pending_eva_cues.push(EvaCue {
            house: Arc::<str>::from(house),
            event,
        });
    }

    /// 取出并清空本 tick 累计的 EVA 提示。
    pub fn take_eva_cues(&mut self) -> Vec<EvaCue> {
        std::mem::take(&mut self.pending_eva_cues)
    }
}
