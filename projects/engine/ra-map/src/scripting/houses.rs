//! `[Houses]` 与各方 House 节。

use ra_assets::IniDocument;

/// 地图一方（战役 / 遭遇均可出现）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MapHouse {
    /// 节名（常为 `Player House` 等）。
    pub name: String,
    /// `Country=`。
    pub country: String,
    /// `TechLevel=`。
    pub tech_level: i32,
    /// `Credits=`（地图单位常为百计资金）。
    pub credits: i32,
    /// `IQ=`。
    pub iq: i32,
    /// `Edge=`。
    pub edge: String,
    /// `PlayerControl=`。
    pub player_control: bool,
    /// `Color=`。
    pub color: String,
    /// `Allies=` 逗号列表。
    pub allies: Vec<String>,
}

/// 解析 `[Houses]` 列表及各方节。
pub fn parse_map_houses(doc: &IniDocument) -> Vec<MapHouse> {
    let Some(list) = doc.section("Houses")
    else {
        return Vec::new();
    };
    let mut out = Vec::new();
    for (_key, name) in list.pairs() {
        let name = name.trim();
        if name.is_empty() {
            continue;
        }
        let Some(sec) = doc.section(name)
        else {
            out.push(MapHouse {
                name: name.to_string(),
                country: String::new(),
                tech_level: 0,
                credits: 0,
                iq: 0,
                edge: String::new(),
                player_control: false,
                color: String::new(),
                allies: Vec::new(),
            });
            continue;
        };
        let allies = sec
            .get("Allies")
            .unwrap_or("")
            .split(',')
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .map(str::to_string)
            .collect();
        out.push(MapHouse {
            name: name.to_string(),
            country: sec.get("Country").unwrap_or("").trim().to_string(),
            tech_level: sec.get("TechLevel").and_then(|v| v.parse().ok()).unwrap_or(0),
            credits: sec.get("Credits").and_then(|v| v.parse().ok()).unwrap_or(0),
            iq: sec.get("IQ").and_then(|v| v.parse().ok()).unwrap_or(0),
            edge: sec.get("Edge").unwrap_or("").trim().to_string(),
            player_control: parse_yes(sec.get("PlayerControl").unwrap_or("")),
            color: sec.get("Color").unwrap_or("").trim().to_string(),
            allies,
        });
    }
    out
}

fn parse_yes(raw: &str) -> bool {
    matches!(raw.trim().to_ascii_lowercase().as_str(), "yes" | "true" | "1")
}
