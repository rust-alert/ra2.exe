//! 壳层 / 局内共用的高速打字机文案显现。
//!
//! 主菜单底栏悬停提示、局内右上角系统任务提示与聊天等，都按「完整字符串已就绪，
//! 再按时长逐步露出可见前缀」处理。时长为 `0` 时整行瞬间出现。

/// 默认打字机总时长（秒）。主菜单底栏与局内消息可共用；设为 `0` 则关闭动画。
pub const DEFAULT_TYPEWRITER_SECS: f64 = 0.4;

/// 一段目标文案的打字机状态（按 Unicode 标量逐字显现）。
#[derive(Debug, Clone)]
pub struct TypewriterText {
    full: String,
    elapsed: f64,
    duration_secs: f64,
}

impl Default for TypewriterText {
    fn default() -> Self {
        Self::new(DEFAULT_TYPEWRITER_SECS)
    }
}

impl TypewriterText {
    /// `duration_secs <= 0` 表示瞬间整行，无打字机。
    pub fn new(duration_secs: f64) -> Self {
        Self {
            full: String::new(),
            elapsed: 0.0,
            duration_secs: duration_secs.max(0.0),
        }
    }

    /// 提交新目标。同文案不重启；换文案则从空前缀重新打字。
    pub fn set_text(&mut self, text: impl Into<String>) {
        let text = text.into();
        if text == self.full {
            return;
        }
        self.full = text;
        self.elapsed = 0.0;
    }

    /// 立刻清空。
    pub fn clear(&mut self) {
        if self.full.is_empty() {
            return;
        }
        self.full.clear();
        self.elapsed = 0.0;
    }

    /// 推进打字机。可见切片变化时返回 `true`。
    pub fn tick(&mut self, dt: f64) -> bool {
        if self.is_instant() || self.full.is_empty() || self.is_complete() {
            return false;
        }
        let before = self.visible_chars();
        self.elapsed = (self.elapsed + dt.max(0.0)).min(self.duration_secs);
        let after = self.visible_chars();
        // 浮点累加可能导致字已满但 elapsed 略小于 duration，此处对齐完成态。
        if after >= self.full.chars().count() {
            self.elapsed = self.duration_secs;
        }
        before != after
    }

    /// 当前应绘制的可见前缀。
    pub fn visible(&self) -> &str {
        let n = self.visible_chars();
        if n == 0 {
            return "";
        }
        let total = self.full.chars().count();
        if n >= total {
            return &self.full;
        }
        match self.full.char_indices().nth(n) {
            Some((idx, _)) => &self.full[..idx],
            None => &self.full,
        }
    }

    /// 是否仍在打字（非瞬间模式且未打完）。
    pub fn is_animating(&self) -> bool {
        !self.is_instant() && !self.full.is_empty() && !self.is_complete()
    }

    fn is_instant(&self) -> bool {
        self.duration_secs <= 0.0
    }

    fn is_complete(&self) -> bool {
        self.is_instant() || self.full.is_empty() || self.elapsed >= self.duration_secs
    }

    fn visible_chars(&self) -> usize {
        let total = self.full.chars().count();
        if total == 0 {
            return 0;
        }
        if self.is_instant() {
            return total;
        }
        if self.elapsed <= 0.0 {
            return 0;
        }
        let t = (self.elapsed / self.duration_secs).clamp(0.0, 1.0);
        let n = (t * total as f64).ceil() as usize;
        n.clamp(1, total)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn zero_duration_is_instant() {
        let mut t = TypewriterText::new(0.0);
        t.set_text("HELLO");
        assert_eq!(t.visible(), "HELLO");
        assert!(!t.tick(1.0));
        assert!(!t.is_animating());
    }

    #[test]
    fn finishes_in_about_point_four_seconds() {
        let mut t = TypewriterText::new(0.4);
        t.set_text("ABCD");
        assert_eq!(t.visible(), "");
        assert!(t.tick(0.05));
        assert!(t.visible().chars().count() >= 1);
        assert!(t.visible().chars().count() < 4);
        assert!(t.tick(0.35));
        assert_eq!(t.visible(), "ABCD");
        assert!(!t.is_animating());
        assert!(!t.tick(0.1));
    }

    #[test]
    fn same_text_keeps_progress() {
        let mut t = TypewriterText::new(0.4);
        t.set_text("XY");
        let _ = t.tick(0.4);
        t.set_text("XY");
        assert_eq!(t.visible(), "XY");
    }

    #[test]
    fn unicode_scalar_steps() {
        let mut t = TypewriterText::new(0.4);
        t.set_text("任务AB");
        let _ = t.tick(0.1);
        assert_eq!(t.visible().chars().count(), 1);
        let _ = t.tick(0.3);
        assert_eq!(t.visible(), "任务AB");
    }
}
