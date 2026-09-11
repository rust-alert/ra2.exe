//! 弹头 `Verses`：相对 11 种护甲的伤害百分比。

use std::ops::{Deref, DerefMut};

/// 对应 [`super::ARMOR_ORDER`] 的 11 项伤害百分比（缺省全 100）。
///
/// 装载期由 INI `Verses=` **一次**解码得到；执行侧只读此类型，不再二次拆分文本。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct WarheadVerses(pub [u32; 11]);

impl WarheadVerses {
    /// 缺省：全部 100%。
    pub const fn all_full() -> Self {
        Self([100; 11])
    }

    /// 内部数组。
    pub const fn as_array(&self) -> &[u32; 11] {
        &self.0
    }
}

impl Default for WarheadVerses {
    fn default() -> Self {
        Self::all_full()
    }
}

impl Deref for WarheadVerses {
    type Target = [u32; 11];

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for WarheadVerses {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl From<[u32; 11]> for WarheadVerses {
    fn from(value: [u32; 11]) -> Self {
        Self(value)
    }
}

impl From<WarheadVerses> for [u32; 11] {
    fn from(value: WarheadVerses) -> Self {
        value.0
    }
}
