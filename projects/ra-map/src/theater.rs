//! 剧院：决定地形 MIX 与等距调色板。

use ra_types::{RaError, RaResult};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Theater {
    Temperate,
    Snow,
    Urban,
    Lunar,
    Desert,
}

impl Theater {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Temperate => "temperate",
            Self::Snow => "snow",
            Self::Urban => "urban",
            Self::Lunar => "lunar",
            Self::Desert => "desert",
        }
    }

    pub fn parse(s: &str) -> RaResult<Self> {
        match s.trim().to_ascii_uppercase().as_str() {
            "TEMPERATE" | "TEM" => Ok(Self::Temperate),
            "SNOW" | "SNO" => Ok(Self::Snow),
            "URBAN" | "URB" => Ok(Self::Urban),
            "LUNAR" | "LUN" => Ok(Self::Lunar),
            "DESERT" | "DES" => Ok(Self::Desert),
            other => Err(RaError::Parse(format!("未知剧院 `{other}`"))),
        }
    }
}

/// 该剧院应挂载的 MIX 名（磁盘或嵌套）。
pub fn theater_mix_names(theater: Theater) -> &'static [&'static str] {
    match theater {
        Theater::Temperate => &["temperat.mix", "tem.mix"],
        Theater::Snow => &["snow.mix", "sno.mix"],
        Theater::Urban => &["urban.mix", "urb.mix"],
        Theater::Lunar => &["lunar.mix", "lun.mix"],
        Theater::Desert => &["desert.mix", "des.mix"],
    }
}

/// 剧院等距地形调色板文件名。
pub fn theater_palette(theater: Theater) -> &'static str {
    match theater {
        Theater::Temperate => "isotem.pal",
        Theater::Snow => "isosno.pal",
        Theater::Urban => "isourb.pal",
        Theater::Lunar => "isolun.pal",
        Theater::Desert => "isodes.pal",
    }
}
