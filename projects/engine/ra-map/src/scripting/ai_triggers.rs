//! `[AITriggerTypes]` 解析（执行后置；有条目则报告能力缺口）。

use ra_assets::IniDocument;

/// 一条 AI 触发（字段子集，供缺口诊断与后续执行）。
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
            let fields: Vec<&str> = value.split(',').map(str::trim).collect();
            out.push(MapAiTrigger {
                id: if key.is_empty() { fields.first().copied().unwrap_or("").to_string() } else { key.to_string() },
                name: fields.first().copied().unwrap_or("").to_string(),
                team: fields.get(1).copied().unwrap_or("").to_string(),
                owner_house: fields.get(2).copied().unwrap_or("").to_string(),
                tech_level: fields.get(3).and_then(|v| v.parse().ok()).unwrap_or(0),
            });
            continue;
        }
        let id = value.to_string();
        if let Some(sec) = doc.section(&id) {
            out.push(MapAiTrigger {
                id: id.clone(),
                name: sec.get("Name").unwrap_or(id.as_str()).trim().to_string(),
                team: first_nonempty(sec.get("Team1").or_else(|| sec.get("Team"))).unwrap_or_default(),
                owner_house: first_nonempty(sec.get("OwnerHouse").or_else(|| sec.get("House"))).unwrap_or_default(),
                tech_level: sec.get("TechLevel").and_then(|v| v.parse().ok()).unwrap_or(0),
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

fn first_nonempty(raw: Option<&str>) -> Option<String> {
    raw.map(str::trim).filter(|s| !s.is_empty()).map(str::to_string)
}
