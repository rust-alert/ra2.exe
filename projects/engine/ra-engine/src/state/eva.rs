//! 玩法侧 EVA 提示队列（按 house 定向，壳层只播本机阵营）。

use std::sync::Arc;

/// 基地遇袭 EVA：同类抑制半径（切比雪夫格距）。
pub(crate) const BASE_UNDER_ATTACK_DEDUP_CELLS: u32 = 8;
/// 基地遇袭 EVA：抑制窗口（逻辑 tick；约 40s @ 15Hz）。
pub(crate) const BASE_UNDER_ATTACK_SUPPRESS_TICKS: u32 = 600;
/// 雷达事件在队列中保留时长（逻辑 tick；约 60s @ 15Hz）。
pub(crate) const RADAR_EVENT_TTL_TICKS: u64 = 900;
/// 每阵营最多保留的雷达事件条数。
pub(crate) const RADAR_EVENT_CAP_PER_HOUSE: usize = 8;

/// 一条应对某阵营播放的 EVA 事件。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EvaCue {
    /// 应收听的阵营 house 名。
    pub house: Arc<str>,
    /// `eva.ini` / `evamd.ini` 事件 id（如 `EVA_UnitReady`）。
    pub event: &'static str,
}

/// 雷达事件（空格跳转 / 小地图闪点；按 house 定向）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RadarEvent {
    /// 应看见该事件的阵营。
    pub house: Arc<str>,
    /// 事件格 X。
    pub x: u16,
    /// 事件格 Y。
    pub y: u16,
    /// 登记时的仿真 tick。
    pub created_tick: u64,
}

/// 基地遇袭播报去重窗口（同 house、近距、未过期则不再排队）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct EvaBaseUnderAttackGate {
    pub house: Arc<str>,
    pub x: u16,
    pub y: u16,
    pub expire_tick: u64,
}

impl crate::state::BattleState {
    /// 向指定阵营排队一条 EVA（空 house / 空事件忽略）。
    pub fn push_eva_cue(&mut self, house: impl AsRef<str>, event: &'static str) {
        let house = house.as_ref().trim();
        if house.is_empty() || event.is_empty() {
            return;
        }
        self.pending_eva_cues.push(EvaCue { house: Arc::<str>::from(house), event });
    }

    /// 取出并清空本 tick 累计的 EVA 提示。
    pub fn take_eva_cues(&mut self) -> Vec<EvaCue> {
        std::mem::take(&mut self.pending_eva_cues)
    }

    /// 向指定阵营登记一条雷达事件（空 house 忽略）。
    pub fn push_radar_event(&mut self, house: impl AsRef<str>, x: u16, y: u16) {
        let house = house.as_ref().trim();
        if house.is_empty() {
            return;
        }
        let tick = self.tick;
        self.expire_radar_events();
        let house_arc = Arc::<str>::from(house);
        self.radar_events.push(RadarEvent { house: house_arc, x, y, created_tick: tick });
        // 同 house 超量时丢最旧。
        let mut kept = 0usize;
        for i in (0..self.radar_events.len()).rev() {
            if self.radar_events[i].house.as_ref().eq_ignore_ascii_case(house) {
                kept += 1;
                if kept > RADAR_EVENT_CAP_PER_HOUSE {
                    self.radar_events.remove(i);
                }
            }
        }
    }

    /// 本机阵营最近一条未过期雷达事件格。
    pub fn last_radar_event_cell(&self, house: &str) -> Option<(u16, u16)> {
        self.radar_event_cells_for(house).last().copied()
    }

    /// 本机阵营全部未过期雷达事件格（旧 → 新）。
    pub fn radar_event_cells_for(&self, house: &str) -> Vec<(u16, u16)> {
        let tick = self.tick;
        self.radar_events
            .iter()
            .filter(|e| e.house.as_ref().eq_ignore_ascii_case(house) && tick.saturating_sub(e.created_tick) < RADAR_EVENT_TTL_TICKS)
            .map(|e| (e.x, e.y))
            .collect()
    }

    /// 清掉过期雷达事件。
    pub(crate) fn expire_radar_events(&mut self) {
        let tick = self.tick;
        self.radar_events.retain(|e| tick.saturating_sub(e.created_tick) < RADAR_EVENT_TTL_TICKS);
    }

    /// 建筑受击时尝试排队 `EVA_OurBaseIsUnderAttack`（近距 + 时间窗去重）。
    pub(crate) fn try_announce_base_under_attack(&mut self, house: &str, x: u16, y: u16) {
        let house = house.trim();
        if house.is_empty() {
            return;
        }
        let tick = self.tick;
        self.eva_base_under_attack.retain(|g| g.expire_tick > tick);
        let dominated = self
            .eva_base_under_attack
            .iter()
            .any(|g| g.house.as_ref().eq_ignore_ascii_case(house) && chebyshev_cells(g.x, g.y, x, y) < BASE_UNDER_ATTACK_DEDUP_CELLS);
        if dominated {
            return;
        }
        self.push_eva_cue(house, "EVA_OurBaseIsUnderAttack");
        self.push_radar_event(house, x, y);
        self.eva_base_under_attack.push(EvaBaseUnderAttackGate {
            house: Arc::<str>::from(house),
            x,
            y,
            expire_tick: tick.saturating_add(u64::from(BASE_UNDER_ATTACK_SUPPRESS_TICKS)),
        });
    }
}

fn chebyshev_cells(ax: u16, ay: u16, bx: u16, by: u16) -> u32 {
    let dx = (i32::from(ax) - i32::from(bx)).unsigned_abs();
    let dy = (i32::from(ay) - i32::from(by)).unsigned_abs();
    dx.max(dy)
}
