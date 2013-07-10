//! 确定性世界推进用的定点数辅助。

/// 16.16 定点，存于 i32。
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
pub struct Fixed16(pub i32);

impl Fixed16 {
    /// 零。
    pub const ZERO: Self = Self(0);
    /// 整数 1.0。
    pub const ONE: Self = Self(1 << 16);

    /// 由整数构造（左移 16 位）。
    pub fn from_i32(v: i32) -> Self {
        Self(v << 16)
    }

    /// 截断为整数部分。
    pub fn to_i32_trunc(self) -> i32 {
        self.0 >> 16
    }

    /// 饱和加法。
    pub fn saturating_add(self, rhs: Self) -> Self {
        Self(self.0.saturating_add(rhs.0))
    }

    /// 饱和减法。
    pub fn saturating_sub(self, rhs: Self) -> Self {
        Self(self.0.saturating_sub(rhs.0))
    }
}
