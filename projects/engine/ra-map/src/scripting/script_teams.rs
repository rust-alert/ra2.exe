//! `[TaskForces]` / `[ScriptTypes]` / `[TeamTypes]`。

use ra_assets::IniDocument;

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
    /// 动作码（原版 ScriptTypes；例如 `1` = 攻击航点，`3` = 移动到航点，`6` = 部署，`8` = 跳转步骤）。
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
    /// `Max=`。
    pub max: i32,
    /// `Priority=`。
    pub priority: i32,
    /// `VeteranLevel=`。
    pub veteran_level: i32,
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
        let mut entries = Vec::new();
        for i in 0..6 {
            let Some(raw) = sec.get(&i.to_string())
            else {
                continue;
            };
            let fields: Vec<&str> = raw.split(',').map(str::trim).collect();
            if fields.len() < 2 {
                continue;
            }
            entries.push(MapTaskForceEntry {
                count: fields[0].parse().unwrap_or(1),
                type_id: fields[1].to_ascii_uppercase(),
            });
        }
        out.push(MapTaskForce {
            id,
            name: sec.get("Name").unwrap_or("").trim().to_string(),
            entries,
            group: sec.get("Group").and_then(|v| v.parse().ok()).unwrap_or(-1),
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
        let mut steps = Vec::new();
        for i in 0..50 {
            let Some(raw) = sec.get(&i.to_string())
            else {
                continue;
            };
            let fields: Vec<&str> = raw.split(',').map(str::trim).collect();
            if fields.len() < 2 {
                continue;
            }
            steps.push(MapScriptStep {
                action: fields[0].parse().unwrap_or(0),
                argument: fields[1].parse().unwrap_or(0),
            });
        }
        out.push(MapScriptType {
            id,
            name: sec.get("Name").unwrap_or("").trim().to_string(),
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
        out.push(MapTeamType {
            id,
            name: sec.get("Name").unwrap_or("").trim().to_string(),
            house: sec.get("House").unwrap_or("").trim().to_string(),
            script: sec.get("Script").unwrap_or("").trim().to_string(),
            task_force: sec.get("TaskForce").unwrap_or("").trim().to_string(),
            tag: sec.get("Tag").unwrap_or("").trim().to_string(),
            max: sec.get("Max").and_then(|v| v.parse().ok()).unwrap_or(0),
            priority: sec.get("Priority").and_then(|v| v.parse().ok()).unwrap_or(0),
            veteran_level: sec.get("VeteranLevel").and_then(|v| v.parse().ok()).unwrap_or(0),
        });
    }
    out
}

fn list_ids(doc: &IniDocument, section: &str) -> Vec<String> {
    let Some(sec) = doc.section(section)
    else {
        return Vec::new();
    };
    sec.pairs()
        .map(|(_, v)| v.trim().to_string())
        .filter(|s| !s.is_empty())
        .collect()
}
