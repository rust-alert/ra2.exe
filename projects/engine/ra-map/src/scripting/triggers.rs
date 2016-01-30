//! `[Tags]` / `[Triggers]` / `[Events]` / `[Actions]` / `[CellTags]`。

use ra_assets::IniDocument;

/// `[Tags]` 一行。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MapTag {
    /// Tag id。
    pub id: String,
    /// 持久性：0 volatile / 1 semi / 2 persistent。
    pub persistence: u8,
    /// 编辑器名。
    pub name: String,
    /// 关联 Trigger id。
    pub trigger_id: String,
}

/// `[Triggers]` 一行。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MapTrigger {
    /// Trigger id。
    pub id: String,
    /// 所属 house。
    pub house: String,
    /// 链接的另一 trigger（`<none>` 表示无）。
    pub linked: String,
    /// 编辑器名。
    pub name: String,
    /// `1` = 初始禁用。
    pub disabled: bool,
    /// Easy 难度启用。
    pub easy: bool,
    /// Normal 难度启用。
    pub normal: bool,
    /// Hard 难度启用。
    pub hard: bool,
}

/// 单条事件条件。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MapEventCondition {
    /// 事件类型码（原版 Events；例如 `13` = 计时结束，`1` = 进入区域）。
    pub kind: i32,
    /// 参数（通常 2 个 int；变长事件保留原文参数）。
    pub params: Vec<String>,
}

/// `[Events]` 中与某 trigger 对齐的事件表。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MapEvent {
    /// Trigger id。
    pub id: String,
    /// 条件列表。
    pub conditions: Vec<MapEventCondition>,
}

/// 单条动作。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MapActionCommand {
    /// 动作类型码（原版 Actions；例如 `1` = Win，`4` = Create Team，`5` = Destroy Team，`32` = Destroy Attached）。
    pub kind: i32,
    /// 七个参数槽（含航点字母等）。
    pub params: [String; 7],
}

/// `[Actions]` 中与某 trigger 对齐的动作表。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MapAction {
    /// Trigger id。
    pub id: String,
    /// 动作列表。
    pub commands: Vec<MapActionCommand>,
}

/// `[CellTags]`：格子绑定 Tag。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MapCellTag {
    /// 格子 X。
    pub x: u16,
    /// 格子 Y。
    pub y: u16,
    /// Tag id。
    pub tag_id: String,
}

/// 解析 `[Tags]`。
pub fn parse_tags(doc: &IniDocument) -> Vec<MapTag> {
    let Some(sec) = doc.section("Tags")
    else {
        return Vec::new();
    };
    let mut out = Vec::new();
    for (id, value) in sec.pairs() {
        let fields: Vec<&str> = value.split(',').map(str::trim).collect();
        if fields.len() < 3 {
            continue;
        }
        out.push(MapTag {
            id: id.to_string(),
            persistence: fields[0].parse().unwrap_or(0),
            name: fields[1].to_string(),
            trigger_id: fields[2].to_string(),
        });
    }
    out
}

/// 解析 `[Triggers]`。
pub fn parse_triggers(doc: &IniDocument) -> Vec<MapTrigger> {
    let Some(sec) = doc.section("Triggers")
    else {
        return Vec::new();
    };
    let mut out = Vec::new();
    for (id, value) in sec.pairs() {
        let fields: Vec<&str> = value.split(',').map(str::trim).collect();
        if fields.len() < 7 {
            continue;
        }
        out.push(MapTrigger {
            id: id.to_string(),
            house: fields[0].to_string(),
            linked: fields[1].to_string(),
            name: fields[2].to_string(),
            disabled: fields[3] == "1",
            easy: fields[4] != "0",
            normal: fields.get(5).map(|v| *v != "0").unwrap_or(true),
            hard: fields.get(6).map(|v| *v != "0").unwrap_or(true),
        });
    }
    out
}

/// 解析 `[Events]`（每条件默认 3 字段：kind,p1,p2）。
pub fn parse_events(doc: &IniDocument) -> Vec<MapEvent> {
    let Some(sec) = doc.section("Events")
    else {
        return Vec::new();
    };
    let mut out = Vec::new();
    for (id, value) in sec.pairs() {
        let fields: Vec<&str> = value.split(',').map(str::trim).collect();
        if fields.is_empty() {
            continue;
        }
        let count: usize = fields[0].parse().unwrap_or(0);
        let mut conditions = Vec::new();
        let mut idx = 1usize;
        for _ in 0..count {
            if idx >= fields.len() {
                break;
            }
            let kind: i32 = fields[idx].parse().unwrap_or(0);
            idx += 1;
            let mut params = Vec::new();
            for _ in 0..2 {
                if idx < fields.len() {
                    params.push(fields[idx].to_string());
                    idx += 1;
                } else {
                    params.push(String::new());
                }
            }
            conditions.push(MapEventCondition { kind, params });
        }
        out.push(MapEvent { id: id.to_string(), conditions });
    }
    out
}

/// 解析 `[Actions]`（每动作 8 字段：kind + 7 params）。
pub fn parse_actions(doc: &IniDocument) -> Vec<MapAction> {
    let Some(sec) = doc.section("Actions")
    else {
        return Vec::new();
    };
    let mut out = Vec::new();
    for (id, value) in sec.pairs() {
        let fields: Vec<&str> = value.split(',').map(str::trim).collect();
        if fields.is_empty() {
            continue;
        }
        let count: usize = fields[0].parse().unwrap_or(0);
        let mut commands = Vec::new();
        let mut idx = 1usize;
        for _ in 0..count {
            if idx >= fields.len() {
                break;
            }
            let kind: i32 = fields[idx].parse().unwrap_or(0);
            idx += 1;
            let mut params = [
                String::new(),
                String::new(),
                String::new(),
                String::new(),
                String::new(),
                String::new(),
                String::new(),
            ];
            for p in params.iter_mut() {
                if idx < fields.len() {
                    *p = fields[idx].to_string();
                    idx += 1;
                }
            }
            commands.push(MapActionCommand { kind, params });
        }
        out.push(MapAction { id: id.to_string(), commands });
    }
    out
}

/// 解析 `[CellTags]`（键 = `y * 1000 + x`）。
pub fn parse_cell_tags(doc: &IniDocument) -> Vec<MapCellTag> {
    let Some(sec) = doc.section("CellTags")
    else {
        return Vec::new();
    };
    let mut out = Vec::new();
    for (packed, tag_id) in sec.pairs() {
        let Ok(n) = packed.parse::<u32>()
        else {
            continue;
        };
        let y = (n / 1000) as u16;
        let x = (n % 1000) as u16;
        out.push(MapCellTag {
            x,
            y,
            tag_id: tag_id.trim().to_string(),
        });
    }
    out
}
