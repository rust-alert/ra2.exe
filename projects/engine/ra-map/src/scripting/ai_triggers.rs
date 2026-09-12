//! `[AITriggerTypes]` 解析（引擎侧 `tick_ai_triggers` 最小执行产队）。

use ra_assets::{IniDocument, from_csv_row, parse_westwood_csv_line};
use serde::Deserialize;

/// 一条 AI 触发（装载解析中间态；投影进 `ra_types::MapAiTrigger`）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MapAiTrigger {
    /// 触发 id（列表键或节名）。
    pub id: String,
    /// 显示名。
    pub name: String,
    /// 关联 TeamType。
    pub team: String,
    /// 所属 House。
    pub owner_house: String,
    /// 科技等级门槛。
    pub tech_level: i32,
}

#[derive(Debug, Deserialize)]
struct AiTriggerCsvRow {
    name: String,
    #[serde(default)]
    team: String,
    #[serde(default)]
    owner_house: String,
    #[serde(default)]
    tech_level: i32,
}

#[derive(Debug, Default, Deserialize)]
struct AiTriggerSectionFields {
    #[serde(rename = "Name")]
    name: Option<String>,
    #[serde(rename = "Team1")]
    team1: Option<String>,
    #[serde(rename = "Team")]
    team: Option<String>,
    #[serde(rename = "OwnerHouse")]
    owner_house: Option<String>,
    #[serde(rename = "House")]
    house: Option<String>,
    #[serde(rename = "TechLevel")]
    tech_level: Option<i32>,
}

/// 解析 `[AITriggerTypes]`。
///
/// 支持两种常见写法：`id=Name,Team,House,Tech,...` 行内 CSV，以及 `0=AI1` + `[AI1]` 分节。
pub fn parse_ai_triggers(doc: &IniDocument) -> Vec<MapAiTrigger> {
    let Some(list) = doc.section("AITriggerTypes")
    else {
        return Vec::new();
    };
    let mut out = Vec::new();
    for (key, value) in list.pairs() {
        let key = key.trim();
        let value = value.trim();
        if value.is_empty() {
            continue;
        }
        if value.contains(',') {
            let Ok(row) = from_csv_row::<AiTriggerCsvRow>(&parse_westwood_csv_line(value))
            else {
                continue;
            };
            out.push(MapAiTrigger {
                id: if key.is_empty() { row.name.clone() } else { key.to_string() },
                name: row.name,
                team: row.team,
                owner_house: row.owner_house,
                tech_level: row.tech_level,
            });
            continue;
        }
        let id = value.to_string();
        if let Some(sec) = doc.section(&id) {
            let fields = sec.deserialize::<AiTriggerSectionFields>().unwrap_or_default();
            out.push(MapAiTrigger {
                id: id.clone(),
                name: fields.name.unwrap_or(id).trim().to_string(),
                team: first_nonempty(fields.team1.or(fields.team)).unwrap_or_default(),
                owner_house: first_nonempty(fields.owner_house.or(fields.house)).unwrap_or_default(),
                tech_level: fields.tech_level.unwrap_or(0),
            });
        }
        else {
            out.push(MapAiTrigger {
                id,
                name: String::new(),
                team: String::new(),
                owner_house: String::new(),
                tech_level: 0,
            });
        }
    }
    out
}

fn first_nonempty(raw: Option<String>) -> Option<String> {
    raw.map(|s| s.trim().to_string()).filter(|s| !s.is_empty())
}
