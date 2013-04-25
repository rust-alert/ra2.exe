//! 陆地类型（TMP `terrain_type`）与地面通行粗判。

/// 零售 TMP 头中的陆地类型字节（与常见 TS/RA2 表对齐的粗枚举）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum LandType {
    Clear = 0,
    Rough = 1,
    Road = 2,
    Water = 3,
    Rock = 4,
    Wall = 5,
    Tiberium = 6,
    Beach = 7,
    Ice = 8,
    Railroad = 9,
}

impl LandType {
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn water_and_rock_block_ground() {
        assert!(!ground_passable(3));
        assert!(!ground_passable(4));
        assert!(!ground_passable(5));
        assert!(ground_passable(0));
        assert!(ground_passable(2));
        assert!(ground_passable(255));
    }
}
