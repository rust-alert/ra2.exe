//! `[TaskForces]` / `[ScriptTypes]` / `[TeamTypes]`。

use ra_assets::{IniDocument, from_csv_row, numbered_pairs, parse_westwood_csv_line};
use ra_types::{HouseName, ScriptTypeName, TagName, TaskForceName, TeamTypeName, TechnoName};
use serde::Deserialize;

/// TaskForce 成员槽（装载解析中间态；投影进 `ra_types::MapTaskForceEntry`）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MapTaskForceEntry {
    /// 数量。
    pub count: u16,
    /// 类型 id（装载期一次解码为大写 techno 键）。
    pub type_id: TechnoName,
}

/// `[TaskForces]` 一项（装载解析中间态；投影进 `ra_types::MapTaskForce`）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MapTaskForce {
    /// id（装载期一次解码为大写 TaskForces 键）。
    pub id: TaskForceName,
    /// 名称。
    pub name: String,
    /// 成员（最多 6）。
    pub entries: Vec<MapTaskForceEntry>,
    /// `Group=`。
    pub group: i32,
}

/// Script 一步（装载解析中间态；投影进 `ra_types::MapScriptStep`）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MapScriptStep {
    /// 动作码（原版 ScriptTypes；例如 `1` = 攻击航点，`3` = 移动，`6` = 部署，`7` = 驻守，`8` = 跳转）。
    pub action: i32,
    /// 参数（含义随 `action`；例如航点编号）。
    pub argument: i32,
}

/// `[ScriptTypes]` 一项（装载解析中间态；投影进 `ra_types::MapScriptType`）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MapScriptType {
    /// id（装载期一次解码为大写 ScriptTypes 键）。
    pub id: ScriptTypeName,
    /// 名称。
    pub name: String,
    /// 步骤。
    pub steps: Vec<MapScriptStep>,
}

/// `[TeamTypes]` 一项（装载解析中间态；投影进 `ra_types::MapTeamType`）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MapTeamType {
    /// id（装载期一次解码为大写 TeamTypes 键）。
    pub id: TeamTypeName,
    /// 名称。
    pub name: String,
    /// `House=`（装载期一次解码为大写）。
    pub house: HouseName,
    /// `Script=`（装载期一次解码为大写 ScriptTypes 键）。
    pub script: ScriptTypeName,
    /// `TaskForce=`（装载期一次解码为大写 TaskForces 键）。
    pub task_force: TaskForceName,
    /// `Tag=`（装载期一次解码为大写 Tags 键；可空）。
    pub tag: TagName,
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
    #[serde(rename = "House", default)]
    house: HouseName,
    #[serde(rename = "Script", default)]
    script: ScriptTypeName,
    #[serde(rename = "TaskForce", default)]
    task_force: TaskForceName,
    #[serde(rename = "Tag", default)]
    tag: TagName,
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
    type_id: TechnoName,
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
        for (index, raw) in numbered_pairs(sec) {
            if index >= 6 {
                break;
            }
            let Ok(row) = from_csv_row::<TaskForceEntryRow>(&parse_westwood_csv_line(raw))
            else {
                continue;
            };
            entries.push(MapTaskForceEntry {
                count: row.count.max(1),
                type_id: row.type_id,
            });
        }
        out.push(MapTaskForce {
            id: TaskForceName::parse(&id),
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
        for (index, raw) in numbered_pairs(sec) {
            if index >= 50 {
                break;
            }
            let Ok(row) = from_csv_row::<ScriptStepRow>(&parse_westwood_csv_line(raw))
            else {
                continue;
            };
            steps.push(MapScriptStep { action: row.action, argument: row.argument });
        }
        out.push(MapScriptType {
            id: ScriptTypeName::parse(&id),
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
            id: TeamTypeName::parse(&id),
            name: fields.name.unwrap_or_default().trim().to_string(),
            house: fields.house,
            script: fields.script,
            task_force: fields.task_force,
            tag: fields.tag,
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
    numbered_pairs(sec)
        .into_iter()
        .map(|(_, v)| v.trim().to_string())
        .filter(|s| !s.is_empty())
        .collect()
}
