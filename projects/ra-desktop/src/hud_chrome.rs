//! 对局页占位 HUD 色块几何（由 `HudSnapshot` 驱动）。
//!
//! **不是 Pre-Alpha 原版 HUD 交付。** 色块只证明资金/电力已进入画面叠层；
//! 正式侧栏、字体与 SHP 另接。标题栏经济文案仍可并行存在。

use ra_engine::HudSnapshot;
use ra_renderer::ScreenChromeQuad;

/// 本地玩家资金条满幅参考（仅占位比例，非规则真值）。
const FUNDS_BAR_REF: f32 = 10_000.0;

/// 由 HUD 快照生成屏上色块（右缘侧栏 + 资金/电力条）。
///
/// `local_house` 为本地阵营名；缺失时画空侧栏框，避免无数据时完全空白。
pub fn match_hud_chrome(hud: &HudSnapshot, local_house: Option<&str>) -> Vec<ScreenChromeQuad> {
    let mut quads = Vec::with_capacity(6);
    // 右侧占位侧栏底。
    quads.push(ScreenChromeQuad {
        x0: 0.84,
        y0: 0.0,
        x1: 1.0,
        y1: 1.0,
        color: [0.08, 0.10, 0.14, 0.88],
    });
    // 顶条：暂停 / 结算提示色。
    if hud.outcome.is_some() {
        quads.push(ScreenChromeQuad {
            x0: 0.84,
            y0: 0.0,
            x1: 1.0,
            y1: 0.08,
            color: [0.55, 0.45, 0.12, 0.95],
        });
    }
    else if hud.paused {
        quads.push(ScreenChromeQuad {
            x0: 0.84,
            y0: 0.0,
            x1: 1.0,
            y1: 0.08,
            color: [0.35, 0.35, 0.40, 0.95],
        });
    }

    let local = local_house.and_then(|house| hud.players.iter().find(|p| p.house.as_ref() == house));
    let Some(player) = local
    else {
        return quads;
    };

    // 资金条（绿）。
    let funds_ratio = (player.funds as f32 / FUNDS_BAR_REF).clamp(0.05, 1.0);
    quads.push(ScreenChromeQuad {
        x0: 0.86,
        y0: 0.12,
        x1: 0.98,
        y1: 0.16,
        color: [0.12, 0.12, 0.12, 0.9],
    });
    quads.push(ScreenChromeQuad {
        x0: 0.86,
        y0: 0.12,
        x1: 0.86 + 0.12 * funds_ratio,
        y1: 0.16,
        color: [0.20, 0.75, 0.28, 0.95],
    });

    // 电力条（黄 / 低电红）。
    let power_den = player.power_drain.max(1) as f32;
    let power_ratio = (player.power_output as f32 / power_den).clamp(0.05, 1.0);
    let power_color = if player.low_power {
        [0.85, 0.22, 0.18, 0.95]
    }
    else {
        [0.90, 0.78, 0.20, 0.95]
    };
    quads.push(ScreenChromeQuad {
        x0: 0.86,
        y0: 0.20,
        x1: 0.98,
        y1: 0.24,
        color: [0.12, 0.12, 0.12, 0.9],
    });
    quads.push(ScreenChromeQuad {
        x0: 0.86,
        y0: 0.20,
        x1: 0.86 + 0.12 * power_ratio.min(1.0),
        y1: 0.24,
        color: power_color,
    });

    // 生产队列占位块。
    if !hud.produce_queues.is_empty() {
        quads.push(ScreenChromeQuad {
            x0: 0.86,
            y0: 0.30,
            x1: 0.98,
            y1: 0.42,
            color: [0.18, 0.28, 0.42, 0.92],
        });
    }

    quads
}

/// 结算页占位动作（色块可点，非原版按钮）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ResultsHit {
    /// 重开遭遇战。
    Rematch,
    /// 返回遭遇战大厅。
    ToLobby,
}

/// 结算页占位按钮命中框（与 [`results_chrome`] 几何一致）。
pub const RESULTS_REMATCH_HIT: (f32, f32, f32, f32) = (0.30, 0.38, 0.70, 0.48);
/// 返回大厅命中框。
pub const RESULTS_LOBBY_HIT: (f32, f32, f32, f32) = (0.30, 0.52, 0.70, 0.62);

/// 结算页占位按钮色块（重开 / 返回大厅）。
pub fn results_chrome() -> Vec<ScreenChromeQuad> {
    let (rx0, ry0, rx1, ry1) = RESULTS_REMATCH_HIT;
    let (lx0, ly0, lx1, ly1) = RESULTS_LOBBY_HIT;
    vec![
        ScreenChromeQuad {
            x0: rx0,
            y0: ry0,
            x1: rx1,
            y1: ry1,
            color: [0.16, 0.42, 0.22, 0.92],
        },
        ScreenChromeQuad {
            x0: lx0,
            y0: ly0,
            x1: lx1,
            y1: ly1,
            color: [0.32, 0.22, 0.18, 0.92],
        },
    ]
}

/// 窗口像素点击 → 结算占位动作。
pub fn hit_results(cursor_x: f64, cursor_y: f64, win_w: f64, win_h: f64) -> Option<ResultsHit> {
    if win_w <= 0.0 || win_h <= 0.0 {
        return None;
    }
    let nx = (cursor_x / win_w) as f32;
    let ny = (cursor_y / win_h) as f32;
    let in_hit = |b: (f32, f32, f32, f32)| nx >= b.0 && nx <= b.2 && ny >= b.1 && ny <= b.3;
    if in_hit(RESULTS_REMATCH_HIT) {
        Some(ResultsHit::Rematch)
    }
    else if in_hit(RESULTS_LOBBY_HIT) {
        Some(ResultsHit::ToLobby)
    }
    else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ra_engine::{HudSnapshot, SnapshotPlayer};
    use std::sync::Arc;

    fn sample_hud(low_power: bool) -> HudSnapshot {
        HudSnapshot {
            tick: 10,
            players: vec![SnapshotPlayer {
                house: Arc::from("Americans"),
                funds: 5000,
                power_output: 50,
                power_drain: if low_power { 100 } else { 40 },
                low_power,
            }],
            produce_queues: vec![],
            last_rejects: vec![],
            outcome: None,
            paused: false,
            pause_reason: None,
            match_stats: None,
        }
    }

    #[test]
    fn match_hud_includes_sidebar_and_bars() {
        let quads = match_hud_chrome(&sample_hud(false), Some("Americans"));
        assert!(quads.len() >= 5);
        assert!((quads[0].x0 - 0.84).abs() < f32::EPSILON);
    }

    #[test]
    fn low_power_uses_red_bar() {
        let quads = match_hud_chrome(&sample_hud(true), Some("Americans"));
        let power_fill = quads.iter().find(|q| (q.y0 - 0.20).abs() < 0.001 && q.color[0] > 0.5).unwrap();
        assert!(power_fill.color[0] > power_fill.color[1]);
    }

    #[test]
    fn results_chrome_has_two_buttons() {
        assert_eq!(results_chrome().len(), 2);
    }

    #[test]
    fn results_hit_maps_rematch_and_lobby() {
        assert_eq!(hit_results(500.0, 430.0, 1000.0, 1000.0), Some(ResultsHit::Rematch));
        assert_eq!(hit_results(500.0, 570.0, 1000.0, 1000.0), Some(ResultsHit::ToLobby));
        assert_eq!(hit_results(50.0, 50.0, 1000.0, 1000.0), None);
    }
}
