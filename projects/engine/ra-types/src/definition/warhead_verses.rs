//! 弹头 `Verses`：相对护甲槽的伤害倍率与索敌资格标志。

use std::ops::{Deref, DerefMut};

/// 单槽 Verses 条目（倍率 + 可选 targeting flags）。
///
/// 原版／Ares 文本形如 `100%`、`100%F`、`50%FRP`：`F`=ForceFire，`R`=Retaliate，`P`=PassiveAcquire。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct VersesEntry {
    /// 伤害百分比（100 = 满额）。
    pub multiplier: u32,
    /// 强制攻击该护甲目标。
    pub force_fire: bool,
    /// 允许反击该护甲目标。
    pub retaliate: bool,
    /// 允许被动索敌该护甲目标。
    pub passive_acquire: bool,
}

impl VersesEntry {
    /// 缺省：100%，无额外标志。
    pub const fn full() -> Self {
        Self { multiplier: 100, force_fire: false, retaliate: false, passive_acquire: false }
    }

    /// 仅倍率、无标志。
    pub const fn from_multiplier(multiplier: u32) -> Self {
        Self { multiplier, force_fire: false, retaliate: false, passive_acquire: false }
    }
}

impl Default for VersesEntry {
    fn default() -> Self {
        Self::full()
    }
}

impl PartialEq<u32> for VersesEntry {
    fn eq(&self, other: &u32) -> bool {
        self.multiplier == *other
    }
}

impl PartialEq<VersesEntry> for u32 {
    fn eq(&self, other: &VersesEntry) -> bool {
        *self == other.multiplier
    }
}

/// 对应 [`super::ARMOR_ORDER`] 的 11 槽 Verses（缺省全 100%）。
///
/// 护甲槽数目前仍对齐原版 11 类；扩展护甲应另走 `ArmorId` 注册表（后续切片）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct WarheadVerses(pub [VersesEntry; 11]);

impl WarheadVerses {
    /// 缺省：全部 100%。
    pub const fn all_full() -> Self {
        Self([VersesEntry::full(); 11])
    }

    /// 仅倍率数组（战斗伤害缩放用）。
    pub fn multipliers(&self) -> [u32; 11] {
        let mut out = [100u32; 11];
        for (i, entry) in self.0.iter().enumerate() {
            out[i] = entry.multiplier;
        }
        out
    }

    /// 由倍率数组构造（标志全关）。
    pub fn from_multipliers(values: [u32; 11]) -> Self {
        let mut out = Self::all_full();
        for (i, v) in values.into_iter().enumerate() {
            out.0[i] = VersesEntry::from_multiplier(v);
        }
        out
    }
}

impl Default for WarheadVerses {
    fn default() -> Self {
        Self::all_full()
    }
}

impl Deref for WarheadVerses {
    type Target = [VersesEntry; 11];

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
        Self::from_multipliers(value)
    }
}

impl From<WarheadVerses> for [u32; 11] {
    fn from(value: WarheadVerses) -> Self {
        value.multipliers()
    }
}
