//! 陆地类型（TMP `terrain_type`）与地面通行粗判。

/// 零售 TMP 头中的陆地类型字节（与常见 TS/RA2 表对齐的粗枚举）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum LandType {
    /// 平地。
    Clear = 0,
    /// 崎岖。
    Rough = 1,
    /// 道路。
    Road = 2,
    /// 水域。
    Water = 3,
    /// 岩石。
    Rock = 4,
    /// 墙。
    Wall = 5,
    /// 矿脉 / 晶体类。
    Tiberium = 6,
    /// 沙滩。
    Beach = 7,
    /// 冰面。
    Ice = 8,
    /// 铁路。
    Railroad = 9,
}

impl LandType {
    /// 由 TMP `terrain_type` 字节解析；未知值返回 `None`。
    pub fn from_u8(v: u8) -> Option<Self> {
        match v {
            0 => Some(Self::Clear),
            1 => Some(Self::Rough),
            2 => Some(Self::Road),
            3 => Some(Self::Water),
            4 => Some(Self::Rock),
            5 => Some(Self::Wall),
            6 => Some(Self::Tiberium),
            7 => Some(Self::Beach),
            8 => Some(Self::Ice),
            9 => Some(Self::Railroad),
            _ => None,
        }
    }
}

/// 地面单位粗判：水 / 岩 / 墙不可走；其余暂可走（坡度另议）。
pub fn ground_passable(terrain_type: u8) -> bool {
    match LandType::from_u8(terrain_type) {
        Some(LandType::Water | LandType::Rock | LandType::Wall) => false,
        Some(_) => true,
        // 未知值保守可走，避免整图封死。
        None => true,
    }
}
