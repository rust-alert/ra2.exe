//! 房屋角色：运行时只认角色，不认零售国名字符串。

/// 房屋在对局中的角色（由 adaptor 从 `[Countries]` / 地图 / 原版兼容默认冻结）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum HouseRole {
    /// 人类或 AI 可玩国家。
    #[default]
    Playable,
    /// 中立环境房屋。
    Neutral,
    /// 平民环境房屋。
    Civilian,
    /// 特殊环境房屋（脚本 / 装饰）。
    Special,
    /// 观察者（不参与作战力量）。
    Observer,
}

impl HouseRole {
    /// 是否为不参与胜负 / 遭遇战 AI 的环境房屋。
    pub const fn is_ambient(self) -> bool {
        matches!(self, Self::Neutral | Self::Civilian | Self::Special)
    }

    /// 原版兼容：仅由已知零售环境房屋名推导角色；未知名返回 `None`（不得当永久语义）。
    pub fn from_stock_ambient_name(name: &str) -> Option<Self> {
        match name.trim().to_ascii_uppercase().as_str() {
            "NEUTRAL" => Some(Self::Neutral),
            "CIVILIAN" => Some(Self::Civilian),
            "SPECIAL" => Some(Self::Special),
            _ => None,
        }
    }
}
