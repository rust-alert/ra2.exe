//! `[Houses]` 与各方 House 节。

use ra_assets::{IniDocument, numbered_pairs};
use ra_types::{ColorName, HouseName, MapEdge};
use serde::Deserialize;

/// 地图一方（装载解析中间态；投影进 `ra_types::MapHouse` 后由运行契约消费）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MapHouse {
    /// 节名（常为 `Player House` 等）。
    pub name: String,
    /// `Country=`（装载期一次解码为大写国家键）。
    pub country: HouseName,
    /// `TechLevel=`。
    pub tech_level: i32,
    /// `Credits=`（地图单位常为百计资金）。
    pub credits: i32,
    /// `IQ=`。
    pub iq: i32,
    /// `Edge=`（装载期一次解码）。
    pub edge: MapEdge,
    /// `PlayerControl=`。
    pub player_control: bool,
    /// `Color=`（装载期一次解码为大写方案名）。
    pub color: ColorName,
    /// `Allies=` 逗号列表。
    pub allies: Vec<String>,
}

/// 单方 House 节字段（一次 Serde）。
#[derive(Debug, Default, Deserialize)]
struct MapHouseSectionFields {
    #[serde(rename = "Country", default)]
    country: HouseName,
    #[serde(rename = "TechLevel")]
    tech_level: Option<i32>,
    #[serde(rename = "Credits")]
    credits: Option<i32>,
    #[serde(rename = "IQ")]
    iq: Option<i32>,
    #[serde(rename = "Edge", default)]
    edge: MapEdge,
    #[serde(rename = "PlayerControl")]
    player_control: Option<bool>,
    #[serde(rename = "Color", default)]
    color: ColorName,
    #[serde(rename = "Allies", default)]
    allies: Vec<String>,
}

impl MapHouseSectionFields {
    fn into_house(self, name: String) -> MapHouse {
        MapHouse {
            name,
            country: self.country,
            tech_level: self.tech_level.unwrap_or(0),
            credits: self.credits.unwrap_or(0),
            iq: self.iq.unwrap_or(0),
            edge: self.edge,
            player_control: self.player_control.unwrap_or(false),
            color: self.color,
            allies: self
                .allies
                .into_iter()
                .map(|s| s.trim().to_string())
                .filter(|s| !s.is_empty())
                .collect(),
        }
    }
}

/// 解析 `[Houses]` 列表及各方节。
pub fn parse_map_houses(doc: &IniDocument) -> Vec<MapHouse> {
    let Some(list) = doc.section("Houses")
    else {
        return Vec::new();
    };
    let mut out = Vec::new();
    for (_index, name) in numbered_pairs(list) {
        let name = name.trim();
        if name.is_empty() {
            continue;
        }
        let Some(sec) = doc.section(name)
        else {
            out.push(MapHouseSectionFields::default().into_house(name.to_string()));
            continue;
        };
        let fields = sec.deserialize::<MapHouseSectionFields>().unwrap_or_default();
        out.push(fields.into_house(name.to_string()));
    }
    out
}
