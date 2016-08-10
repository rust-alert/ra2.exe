//! 战役页交互。

use ra_layout::{RectPx, rect_px_from_snapshot, solve_campaign};

use super::{Shell, campaign_difficulty_from_track_x};

impl Shell {
    /// 战役难度滑条按下：按轨坐标落档并开始拖动。
    pub(super) fn handle_campaign_press(&mut self) -> bool {
        let snap = solve_campaign();
        let track = rect_px_from_snapshot(&snap, "difficulty");
        let label = rect_px_from_snapshot(&snap, "difficulty_label");
        let value = rect_px_from_snapshot(&snap, "difficulty_value");
        let (x, y) = self.shell_cursor_px();
        if !(track.contains(x, y) || label.contains(x, y) || value.contains(x, y)) {
            return false;
        }
        self.campaign_dragging = true;
        self.campaign_pointer_consumed = true;
        self.set_campaign_difficulty_from_x(track, x);
        self.play_menu_click();
        true
    }

    /// 战役难度滑条拖动。
    pub(super) fn handle_campaign_drag(&mut self) -> bool {
        if !self.campaign_dragging {
            return false;
        }
        let track = rect_px_from_snapshot(&solve_campaign(), "difficulty");
        let (x, _) = self.shell_cursor_px();
        self.set_campaign_difficulty_from_x(track, x);
        true
    }

    /// 按轨道 X 映射难度 0..=2，档位变化时刷新。
    pub(super) fn set_campaign_difficulty_from_x(&mut self, track: RectPx, x: i32) {
        let next = campaign_difficulty_from_track_x(track, x);
        if next == self.campaign_difficulty {
            return;
        }
        self.campaign_difficulty = next;
        let label = match next {
            0 => "易",
            2 => "难",
            _ => "中",
        };
        self.banner = format!("战役难度 · {label}");
        self.refresh_menu_backdrop();
        self.refresh_shell_title();
    }
}
