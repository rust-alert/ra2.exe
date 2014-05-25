//! 坐标、朝向与定点数。

/// 16.16 定点，存于 i32（逻辑层优先于 `f32`）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
pub struct Fixed(pub i32);

impl Fixed {
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

/// 旧名别名，与 [`Fixed`] 同一类型。
pub type Fixed16 = Fixed;

/// 地图格坐标。
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
pub struct Cell {
    /// 列。
    pub x: i16,
    /// 行。
    pub y: i16,
}

/// 格内子单元（步兵占位等）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
pub struct SubCell(pub u8);

/// 朝向（0..255 常见于原版约定，具体语义由定义表解释）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
pub struct Facing(pub u8);

/// 逻辑距离（格或定点数，由使用方约定单位）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
pub struct Distance(pub u32);
