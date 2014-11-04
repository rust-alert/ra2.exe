//! 壳层按钮首次绘制 / 切页的 SHP 帧波浪（非整页位移）。
//!
//! 原版 `FUN_006071E0` 按约 30 ms 步进推进各 owner-draw 格的 `SDBTNANM` 帧索引，
//! 形成「先收起再展开」观感：SlideOut 帧上数，SlideIn 帧下数。控件位置不变。

use std::time::{Duration, Instant};

/// 每 tick 间隔（毫秒），对齐原版 `Sleep(0x1E)`。
pub const WAVE_TICK_MS: u32 = 30;
/// 末槽进场后再多跑的尾部 tick，使斜坡收束。
pub const WAVE_TAIL_TICKS: u32 = 6;
/// 斜坡步数（delta `0..=5`）。
pub const WAVE_RAMP_STEPS: i32 = 6;

/// 一组帧常数：`(进场前保持, 斜坡起点, 收束后保持)`。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct WaveFrames {
    /// 该格尚未进入斜坡时的帧。
    pub before: i32,
    /// 斜坡第 0 步的帧。
    pub base: i32,
    /// 斜坡结束后的稳定帧。
    pub after: i32,
}

/// 显示方向（SlideIn）：保持 10 → 斜坡 10‥5 → 收束 1。
pub const GROUP_A_IN: WaveFrames = WaveFrames {
    before: 10,
    base: 10,
    after: 1,
};

/// 关闭方向（SlideOut）：保持 1 → 斜坡 5‥10 → 收束 10。
pub const GROUP_A_OUT: WaveFrames = WaveFrames {
    before: 1,
    base: 5,
    after: 10,
};

/// 波浪方向。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WaveDirection {
    /// 进场：帧号递减。
    SlideIn,
    /// 离场：帧号递增。
    SlideOut,
}

impl WaveDirection {
    fn dir(self) -> i32 {
        match self {
            Self::SlideIn => -1,
            Self::SlideOut => 1,
        }
    }

    fn frames(self) -> WaveFrames {
        match self {
            Self::SlideIn => GROUP_A_IN,
            Self::SlideOut => GROUP_A_OUT,
        }
    }
}

/// 某一壳层页参与波浪的 owner-draw 槽数与交错表。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ShellSlideSpec {
    /// 动画槽数量 `N`（决定总 tick）。
    pub slot_count: u32,
}

/// 主菜单：五个交错档；Exit 与 Options 同档。
pub const MAIN_MENU_SLIDE: ShellSlideSpec = ShellSlideSpec { slot_count: 5 };
/// 单人页：四个右栏钮。
pub const SINGLE_PLAYER_SLIDE: ShellSlideSpec = ShellSlideSpec { slot_count: 4 };
/// 遭遇战大厅：三个右栏钮。
pub const SKIRMISH_SLIDE: ShellSlideSpec = ShellSlideSpec { slot_count: 3 };
/// 战役页：仅「上一页」一钮（侧图不走 `SDBTNANM` 波浪）。
pub const CAMPAIGN_SLIDE: ShellSlideSpec = ShellSlideSpec { slot_count: 1 };
/// 选图页：三个右栏钮。
pub const CHOOSE_MAP_SLIDE: ShellSlideSpec = ShellSlideSpec { slot_count: 3 };

/// 主菜单各入口的进场 tick（Exit 与 Options 同为 5）。
const MAIN_MENU_ENTRY_TICKS: &[(&str, i32)] = &[
    ("single_player", 1),
    ("ww_online", 2),
    ("network", 3),
    ("movies", 4),
    ("options", 5),
    ("exit", 5),
];

/// 按槽位下标取进场 tick（槽 0 → tick 1）。
pub fn entry_tick_for_slot(slot: u32) -> i32 {
    slot as i32 + 1
}

/// 主菜单按入口 id 取进场 tick；未知 id 回退槽序。
pub fn main_menu_entry_tick(entry_id: &str, slot: u32) -> i32 {
    MAIN_MENU_ENTRY_TICKS
        .iter()
        .find_map(|(id, tick)| (*id == entry_id).then_some(*tick))
        .unwrap_or_else(|| entry_tick_for_slot(slot))
}

/// 总 tick = `N + 3`（含雷达锚点档）+ 尾部。
pub fn total_ticks_for(slot_count: u32) -> u32 {
    slot_count + 3 + WAVE_TAIL_TICKS
}

/// 给定全局 tick / 该格进场 tick，返回 `SDBTNANM` 帧号。
pub fn frame_for_tick(tick: i32, entry_tick: i32, direction: WaveDirection) -> u16 {
    let f = direction.frames();
    let delta = tick - entry_tick;
    let frame = if delta < 0 {
        f.before
    } else if delta < WAVE_RAMP_STEPS {
        f.base + delta * direction.dir()
    } else {
        f.after
    };
    frame.max(0) as u16
}

/// 一次进行中的壳层波浪。
#[derive(Debug, Clone)]
pub struct ShellFrameWave {
    last_step_at: Instant,
    tick: u32,
    total_ticks: u32,
    direction: WaveDirection,
    slot_count: u32,
}

impl ShellFrameWave {
    /// 新建进场 / 离场波浪。
    pub fn new(spec: ShellSlideSpec, direction: WaveDirection, now: Instant) -> Self {
        Self {
            last_step_at: now,
            tick: 0,
            total_ticks: total_ticks_for(spec.slot_count),
            direction,
            slot_count: spec.slot_count,
        }
    }

    /// 当前方向。
    pub fn direction(&self) -> WaveDirection {
        self.direction
    }

    /// 当前全局 tick。
    pub fn tick(&self) -> u32 {
        self.tick
    }

    /// 是否已跑完全部 tick。
    pub fn is_complete(&self) -> bool {
        self.tick >= self.total_ticks
    }

    /// 距下一 tick 还需等待的时长（已到期则为 `None`）。
    pub fn time_until_next_step(&self, now: Instant) -> Option<Duration> {
        if self.is_complete() {
            return None;
        }
        let step = Duration::from_millis(u64::from(WAVE_TICK_MS));
        let elapsed = now.duration_since(self.last_step_at);
        if elapsed >= step {
            None
        } else {
            Some(step - elapsed)
        }
    }

    /// 至多推进一 tick（满 30 ms 才动），不跳帧。
    pub fn advance(&mut self, now: Instant) -> bool {
        if self.is_complete() {
            return false;
        }
        let step = Duration::from_millis(u64::from(WAVE_TICK_MS));
        if now.duration_since(self.last_step_at) < step {
            return false;
        }
        self.tick += 1;
        self.last_step_at += step;
        true
    }

    /// 槽位下标对应的当前 `SDBTNANM` 帧。
    pub fn frame_for_slot(&self, slot: u32) -> u16 {
        frame_for_tick(self.tick as i32, entry_tick_for_slot(slot), self.direction)
    }

    /// 主菜单入口 id 对应的当前帧。
    pub fn frame_for_main_menu_entry(&self, entry_id: &str, slot: u32) -> u16 {
        frame_for_tick(
            self.tick as i32,
            main_menu_entry_tick(entry_id, slot),
            self.direction,
        )
    }

    /// 诊断用槽数。
    pub fn slot_count(&self) -> u32 {
        self.slot_count
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn total_ticks_matches_native_table() {
        for (n, total) in [(3u32, 12), (4, 13), (5, 14), (6, 15)] {
            assert_eq!(total_ticks_for(n), total, "N={n}");
        }
    }

    #[test]
    fn slide_in_ramps_down_then_settles() {
        // 槽 0 进场 tick=1：tick1→10 … tick6→5，之后收束 1。
        assert_eq!(frame_for_tick(0, 1, WaveDirection::SlideIn), 10);
        assert_eq!(frame_for_tick(1, 1, WaveDirection::SlideIn), 10);
        assert_eq!(frame_for_tick(2, 1, WaveDirection::SlideIn), 9);
        assert_eq!(frame_for_tick(6, 1, WaveDirection::SlideIn), 5);
        assert_eq!(frame_for_tick(7, 1, WaveDirection::SlideIn), 1);
    }

    #[test]
    fn slide_out_ramps_up_then_settles() {
        assert_eq!(frame_for_tick(0, 1, WaveDirection::SlideOut), 1);
        assert_eq!(frame_for_tick(1, 1, WaveDirection::SlideOut), 5);
        assert_eq!(frame_for_tick(2, 1, WaveDirection::SlideOut), 6);
        assert_eq!(frame_for_tick(6, 1, WaveDirection::SlideOut), 10);
        assert_eq!(frame_for_tick(7, 1, WaveDirection::SlideOut), 10);
    }

    #[test]
    fn main_menu_exit_shares_options_stagger() {
        assert_eq!(main_menu_entry_tick("options", 4), 5);
        assert_eq!(main_menu_entry_tick("exit", 5), 5);
    }

    #[test]
    fn advance_one_tick_per_interval() {
        let start = Instant::now();
        let mut wave = ShellFrameWave::new(MAIN_MENU_SLIDE, WaveDirection::SlideIn, start);
        assert!(!wave.advance(start));
        assert!(wave.advance(start + Duration::from_millis(30)));
        assert_eq!(wave.tick(), 1);
        assert!(!wave.advance(start + Duration::from_millis(45)));
        assert!(wave.advance(start + Duration::from_millis(60)));
        assert_eq!(wave.tick(), 2);
    }
}
