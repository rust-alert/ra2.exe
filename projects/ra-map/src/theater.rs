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
        Theater::Temperate => &["temperat.mix", "tem.mix", "isotemp.mix"],
        Theater::Snow => &["snow.mix", "sno.mix", "isosnow.mix"],
        Theater::Urban => &["urban.mix", "urb.mix", "isourb.mix"],
        Theater::Lunar => &["lunar.mix", "lun.mix", "isolun.mix"],
        Theater::Desert => &["desert.mix", "des.mix", "isodes.mix"],
    }
}

/// 剧院 TMP 文件扩展名（不含点）。
pub fn theater_tmp_extension(theater: Theater) -> &'static str {
    match theater {
        Theater::Temperate => "tem",
        Theater::Snow => "sno",
        Theater::Urban => "urb",
        Theater::Lunar => "lun",
        Theater::Desert => "des",
    }
}

/// 剧院控制 INI 文件名。
pub fn theater_ini_name(theater: Theater) -> &'static str {
    match theater {
        Theater::Temperate => "temperat.ini",
        Theater::Snow => "snow.ini",
        Theater::Urban => "urban.ini",
        Theater::Lunar => "lunar.ini",
        Theater::Desert => "desert.ini",
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

/// `NewTheater=yes` 时替换文件名第二字母所用的剧院字符。
pub fn theater_new_letter(theater: Theater) -> char {
    match theater {
        Theater::Temperate => 't',
        Theater::Snow => 'a',
        Theater::Urban => 'u',
        Theater::Lunar => 'l',
        Theater::Desert => 'd',
    }
}

/// 生成 `NewTheater` 规则下的 SHP 主文件名（小写，含 `.shp`）。
pub fn new_theater_shp_name(stem: &str, theater: Theater) -> String {
    let letter = theater_new_letter(theater);
    let mut chars: Vec<char> = stem.chars().collect();
    if chars.len() >= 2 {
        chars[1] = letter;
    }
    let mut name: String = chars.into_iter().collect();
    name.make_ascii_lowercase();
    name.push_str(".shp");
    name
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn temperate_new_theater_name() {
        assert_eq!(new_theater_shp_name("CAMSC01", Theater::Temperate), "ctmsc01.shp");
        assert_eq!(new_theater_shp_name("CAAIRP", Theater::Temperate), "ctairp.shp");
    }
}
