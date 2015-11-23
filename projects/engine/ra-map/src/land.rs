//! 陆地类型（TMP `terrain_type` → 规范 `LandType`）与地面通行粗判。

/// 规范陆地类型（与零售 `CellClass` LandType 序号一致）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum LandType {
    /// 平地。
    Clear = 0,
    /// 道路。
    Road = 1,
    /// 水域。
    Water = 2,
    /// 岩石。
    Rock = 3,
    /// 墙。
    Wall = 4,
    /// 矿脉 / 晶体类。
    Tiberium = 5,
    /// 沙滩。
    Beach = 6,
    /// 崎岖。
    Rough = 7,
    /// 冰面。
    Ice = 8,
    /// 铁路。
    Railroad = 9,
    /// 隧道。
    Tunnel = 10,
    /// 杂草。
    Weeds = 11,
}

impl LandType {
    /// 由规范陆地类型序号解析；未知值返回 `None`。
    pub fn from_u8(v: u8) -> Option<Self> {
        match v {
            0 => Some(Self::Clear),
            1 => Some(Self::Road),
            2 => Some(Self::Water),
            3 => Some(Self::Rock),
            4 => Some(Self::Wall),
            5 => Some(Self::Tiberium),
            6 => Some(Self::Beach),
            7 => Some(Self::Rough),
            8 => Some(Self::Ice),
            9 => Some(Self::Railroad),
            10 => Some(Self::Tunnel),
            11 => Some(Self::Weeds),
            _ => None,
        }
    }

    /// 由 rules `Land=` 字符串解析（大小写不敏感）。
    pub fn parse_name(raw: &str) -> Option<Self> {
        match raw.trim().to_ascii_lowercase().as_str() {
            "clear" => Some(Self::Clear),
            "road" => Some(Self::Road),
            "water" => Some(Self::Water),
            "rock" => Some(Self::Rock),
            "wall" => Some(Self::Wall),
            "tiberium" | "ore" | "gems" => Some(Self::Tiberium),
            "beach" => Some(Self::Beach),
            "rough" => Some(Self::Rough),
            "ice" => Some(Self::Ice),
            "railroad" => Some(Self::Railroad),
            "tunnel" => Some(Self::Tunnel),
            "weeds" => Some(Self::Weeds),
            _ => None,
        }
    }
}

/// TMP 子砖 `terrain_type` 字节 → 规范 `LandType`（16 项查表）。
///
/// 与零售 `IsometricTileTypeClass::GetSubtileLandType` 表一致：
/// `[Clear, Ice×4, Tunnel, Railroad, Rock×2, Water, Beach, Road×2, Clear, Rough, Rock]`。
pub const TMP_TERRAIN_TO_LAND: [LandType; 16] = [
    LandType::Clear,
    LandType::Ice,
    LandType::Ice,
    LandType::Ice,
    LandType::Ice,
    LandType::Tunnel,
    LandType::Railroad,
    LandType::Rock,
    LandType::Rock,
    LandType::Water,
    LandType::Beach,
    LandType::Road,
    LandType::Road,
    LandType::Clear,
    LandType::Rough,
    LandType::Rock,
];

/// 将 TMP `terrain_type` 字节映射为规范陆地类型；越界按 `Clear`。
pub fn tmp_terrain_to_land_type(tmp_terrain_type: u8) -> LandType {
    TMP_TERRAIN_TO_LAND
        .get(usize::from(tmp_terrain_type))
        .copied()
        .unwrap_or(LandType::Clear)
}

/// 规范陆地类型是否允许地面单位通行。
pub fn land_passable(land: LandType) -> bool {
    !matches!(land, LandType::Water | LandType::Rock | LandType::Wall)
}

/// 地面单位粗判：先把 TMP 字节映到规范陆地，再判水 / 岩 / 墙不可走。
pub fn ground_passable(terrain_type: u8) -> bool {
    land_passable(tmp_terrain_to_land_type(terrain_type))
}
