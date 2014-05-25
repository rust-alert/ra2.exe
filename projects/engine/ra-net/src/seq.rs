//! 命令序号窗口：严格顺序接受（乱序/重复拒绝）。

/// 期望下一序号；仅接受恰好匹配的序号。
#[derive(Debug, Clone)]
pub struct SequenceWindow {
    /// 下一条待接受命令的序号。
    pub next: u32,
}

impl Default for SequenceWindow {
    /// 从序号 0 开始的空窗口。
    fn default() -> Self {
        Self { next: 0 }
    }
}

impl SequenceWindow {
    /// 创建窗口，下一接受序号为 `next`。
    pub fn new(next: u32) -> Self {
        Self { next }
    }

    /// 若 `seq == next` 则推进并返回 `true`。
    pub fn accept(&mut self, seq: u32) -> bool {
        if seq == self.next {
            self.next = self.next.wrapping_add(1);
            true
        }
        else {
            false
        }
    }

    /// 在 wrapping 语义下判断 `seq` 是否已落后于当前窗口（重复或过期）。
    pub fn is_duplicate_or_old(&self, seq: u32) -> bool {
        // 在 wrapping 语义下：seq 已落后于 next（距离在半窗口内视为旧）。
        let behind = self.next.wrapping_sub(seq);
        behind != 0 && behind < (u32::MAX / 2)
    }
}
