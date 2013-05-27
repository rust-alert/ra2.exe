//! 命令序号窗口：严格顺序接受（乱序/重复拒绝）。

/// 期望下一序号；仅接受恰好匹配的序号。
#[derive(Debug, Clone)]
pub struct SequenceWindow {
    pub next: u32,
}

impl Default for SequenceWindow {
    fn default() -> Self {
        Self { next: 0 }
    }
}

impl SequenceWindow {
    pub fn new(next: u32) -> Self {
        Self { next }
    }

    /// 若 `seq == next` 则推进并返回 `true`。
    pub fn accept(&mut self, seq: u32) -> bool {
        if seq == self.next {
            self.next = self.next.wrapping_add(1);
            true
        } else {
            false
        }
    }

    pub fn is_duplicate_or_old(&self, seq: u32) -> bool {
        // 在 wrapping 语义下：seq 已落后于 next（距离在半窗口内视为旧）。
        let behind = self.next.wrapping_sub(seq);
        behind != 0 && behind < (u32::MAX / 2)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn accepts_in_order_only() {
        let mut w = SequenceWindow::default();
        assert!(w.accept(0));
        assert!(!w.accept(0));
        assert!(!w.accept(2));
        assert!(w.accept(1));
        assert_eq!(w.next, 2);
    }
}
