//! `[TaskForces]` / `[ScriptTypes]` / `[TeamTypes]`。

use ra_assets::{IniDocument, from_csv_row, parse_westwood_csv_line};
use serde::Deserialize;

/// TaskForce 成员槽。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MapTaskForceEntry {
    /// 数量。
    pub count: u16,
    /// 类型 id。
    pub type_id: String,
}

/// `[TaskForces]` 一项。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MapTaskForce {
    /// id。
    pub id: String,
    /// 名称。
    pub name: String,
    /// 成员（最多 6）。
    pub entries: Vec<MapTaskForceEntry>,
    /// `Group=`。
    pub group: i32,
}

/// Script 一步。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MapScriptStep {
    /// 动作码（原版 ScriptTypes；例如 `1` = 攻击航点，`3` = 移动，`6` = 部署，`7` = 驻守，`8` = 跳转）。
    pub action: i32,
    /// 参数（含义随 `action`；例如航点编号）。
    pub argument: i32,
}

/// `[ScriptTypes]` 一项。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MapScriptType {
    /// id。
    pub id: String,
    /// 名称。
    pub name: String,
    /// 步骤。
    pub steps: Vec<MapScriptStep>,
}

/// `[TeamTypes]` 一项（字段子集）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MapTeamType {
    /// id。
    pub id: String,
    /// 名称。
    pub name: String,
    /// `House=`。
    pub house: String,
    /// `Script=`。
    pub script: String,
    /// `TaskForce=`。
    pub task_force: String,
    /// `Tag=`（可空）。
    pub tag: String,
    /// `Waypoint=`：产队航点编号；`<0` 表示未指定（运行时回退 index 0）。
    pub waypoint: i32,
    /// `Max=`。
    pub max: i32,
    /// `Priority=`。
    pub priority: i32,
    /// `VeteranLevel=`。
    pub veteran_level: i32,
}

#[derive(Debug, Default, Deserialize)]
struct TeamTypeSectionFields {
    #[serde(rename = "Name")]
    name: Option<String>,
    #[serde(rename = "House")]
    house: Option<String>,
    #[serde(rename = "Script")]
    script: Option<String>,
    #[serde(rename = "TaskForce")]
    task_force: Option<String>,
    #[serde(rename = "Tag")]
    tag: Option<String>,
    #[serde(rename = "Waypoint")]
    waypoint: Option<i32>,
    #[serde(rename = "Max")]
    max: Option<i32>,
    #[serde(rename = "Priority")]
    priority: Option<i32>,
    #[serde(rename = "VeteranLevel")]
    veteran_level: Option<i32>,
}

#[derive(Debug, Default, Deserialize)]
struct NamedGroupSectionFields {
    #[serde(rename = "Name")]
    name: Option<String>,
    #[serde(rename = "Group")]
    group: Option<i32>,
}

#[derive(Debug, Deserialize)]
struct TaskForceEntryRow {
    count: u16,
    type_id: String,
}

#[derive(Debug, Deserialize)]
struct ScriptStepRow {
    action: i32,
    argument: i32,
}

/// 解析全部 TaskForce。
pub fn parse_task_forces(doc: &IniDocument) -> Vec<MapTaskForce> {
    let ids = list_ids(doc, "TaskForces");
    let mut out = Vec::new();
    for id in ids {
        let Some(sec) = doc.section(&id)
        else {
            continue;
        };
        let meta = sec.deserialize::<NamedGroupSectionFields>().unwrap_or_default();
        let mut entries = Vec::new();
        for i in 0..6 {
            let Some(raw) = sec.get(&i.to_string())
            else {
                continue;
            };
            let Ok(row) = from_csv_row::<TaskForceEntryRow>(&parse_westwood_csv_line(raw))
            else {
                continue;
            };
            entries.push(MapTaskForceEntry {
                count: row.count.max(1),
                type_id: row.type_id.to_ascii_uppercase(),
            });
        }
        out.push(MapTaskForce {
            id,
            name: meta.name.unwrap_or_default().trim().to_string(),
            entries,
            group: meta.group.unwrap_or(-1),
        });
    }
    out
}

/// 解析全部 ScriptType。
pub fn parse_script_types(doc: &IniDocument) -> Vec<MapScriptType> {
    let ids = list_ids(doc, "ScriptTypes");
    let mut out = Vec::new();
    for id in ids {
        let Some(sec) = doc.section(&id)
        else {
            continue;
        };
        let meta = sec.deserialize::<NamedGroupSectionFields>().unwrap_or_default();
        let mut steps = Vec::new();
        for i in 0..50 {
            let Some(raw) = sec.get(&i.to_string())
            else {
                continue;
            };
            let Ok(row) = from_csv_row::<ScriptStepRow>(&parse_westwood_csv_line(raw))
            else {
                continue;
            };
            steps.push(MapScriptStep { action: row.action, argument: row.argument });
        }
        out.push(MapScriptType {
            id,
            name: meta.name.unwrap_or_default().trim().to_string(),
            steps,
        });
    }
    out
}

/// 解析全部 TeamType。
pub fn parse_team_types(doc: &IniDocument) -> Vec<MapTeamType> {
    let ids = list_ids(doc, "TeamTypes");
    let mut out = Vec::new();
    for id in ids {
        let Some(sec) = doc.section(&id)
        else {
            continue;
        };
        let fields = sec.deserialize::<TeamTypeSectionFields>().unwrap_or_default();
        out.push(MapTeamType {
            id,
            name: fields.name.unwrap_or_default().trim().to_string(),
            house: fields.house.unwrap_or_default().trim().to_string(),
            script: fields.script.unwrap_or_default().trim().to_string(),
            task_force: fields.task_force.unwrap_or_default().trim().to_string(),
            tag: fields.tag.unwrap_or_default().trim().to_string(),
            waypoint: fields.waypoint.unwrap_or(-1),
            max: fields.max.unwrap_or(0),
            priority: fields.priority.unwrap_or(0),
            veteran_level: fields.veteran_level.unwrap_or(0),
        });
    }
    out
}

fn list_ids(doc: &IniDocument, section: &str) -> Vec<String> {
    let Some(sec) = doc.section(section)
    else {
        return Vec::new();
    };
    sec.pairs().map(|(_, v)| v.trim().to_string()).filter(|s| !s.is_empty()).collect()
}
