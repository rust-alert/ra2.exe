//! `[AITriggerTypes]` 解析（引擎侧 `tick_ai_triggers` 最小执行产队）。

use ra_assets::{IniDocument, parse_westwood_csv_line};
use ra_types::{AiTriggerCompareOp, AiTriggerConditionKind, AiTriggerName, HouseName, TeamTypeName, TechnoName};
use serde::Deserialize;

/// 一条 AI 触发（装载解析中间态；投影进 `ra_types::MapAiTrigger`）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MapAiTrigger {
    /// 触发 id（装载期一次解码为大写 AITriggerTypes 键）。
    pub id: AiTriggerName,
    /// 显示名。
    pub name: String,
    /// 关联 TeamType（装载期一次解码为大写 TeamTypes 键）。
    pub team: TeamTypeName,
    /// 所属 House（装载期一次解码为大写）。
    pub owner_house: HouseName,
    /// 科技等级门槛。
    pub tech_level: i32,
    /// 条件种类。
    pub condition: AiTriggerConditionKind,
    /// 条件对象类型键。
    pub condition_object: TechnoName,
    /// 比较阈值。
    pub compare_amount: i32,
    /// 比较运算符。
    pub compare_op: AiTriggerCompareOp,
    /// 遭遇战可用。
    pub for_skirmish: bool,
    /// Easy 难度启用。
    pub enabled_easy: bool,
    /// Normal 难度启用。
    pub enabled_normal: bool,
    /// Hard 难度启用。
    pub enabled_hard: bool,
    /// 起始权重。
    pub weight: u32,
    /// 可选第二队。
    pub team2: TeamTypeName,
}

impl Default for MapAiTrigger {
    fn default() -> Self {
        Self {
            id: AiTriggerName::default(),
            name: String::new(),
            team: TeamTypeName::default(),
            owner_house: HouseName::default(),
            tech_level: 0,
            condition: AiTriggerConditionKind::Always,
            condition_object: TechnoName::default(),
            compare_amount: 0,
            compare_op: AiTriggerCompareOp::GreaterEqual,
            for_skirmish: true,
            enabled_easy: true,
            enabled_normal: true,
            enabled_hard: true,
            weight: ra_types::DEFAULT_AI_TRIGGER_WEIGHT,
            team2: TeamTypeName::default(),
        }
    }
}

#[derive(Debug, Default, Deserialize)]
struct AiTriggerSectionFields {
    #[serde(rename = "Name")]
    name: Option<String>,
    #[serde(rename = "Team1", default)]
    team1: TeamTypeName,
    #[serde(rename = "Team", default)]
    team: TeamTypeName,
    #[serde(rename = "OwnerHouse", default)]
    owner_house: HouseName,
    #[serde(rename = "House", default)]
    house: HouseName,
    #[serde(rename = "TechLevel")]
    tech_level: Option<i32>,
    #[serde(rename = "Type")]
    condition_type: Option<i32>,
    #[serde(rename = "ConditionType")]
    condition_type_alt: Option<i32>,
    #[serde(rename = "UnitType", default)]
    unit_type: TechnoName,
    #[serde(rename = "ConditionObject", default)]
    condition_object: TechnoName,
    #[serde(rename = "Data", default)]
    data: String,
    #[serde(rename = "Comparator", default)]
    comparator: String,
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
            if let Some(parsed) = parse_ai_trigger_csv_line(key, value) {
                out.push(parsed);
            }
            continue;
        }
        let id = AiTriggerName::parse(value);
        if let Some(sec) = doc.section(id.as_str()) {
            let fields = sec.deserialize::<AiTriggerSectionFields>().unwrap_or_default();
            let owner_house = if !fields.owner_house.is_empty() { fields.owner_house } else { fields.house };
            let name = fields.name.unwrap_or_else(|| id.as_str().to_string()).trim().to_string();
            let cond_ty = fields.condition_type.or(fields.condition_type_alt).unwrap_or(-1);
            let object = if !fields.condition_object.is_empty() { fields.condition_object } else { fields.unit_type };
            let comparator = if !fields.comparator.is_empty() { fields.comparator } else { fields.data };
            let (compare_amount, compare_op) = decode_comparator(&comparator);
            out.push(MapAiTrigger {
                id,
                name,
                team: first_team(fields.team1, fields.team),
                owner_house,
                tech_level: fields.tech_level.unwrap_or(0),
                condition: AiTriggerConditionKind::from_i32(cond_ty),
                condition_object: object,
                compare_amount,
                compare_op,
                for_skirmish: true,
                enabled_easy: true,
                enabled_normal: true,
                enabled_hard: true,
                weight: ra_types::DEFAULT_AI_TRIGGER_WEIGHT,
                team2: TeamTypeName::default(),
            });
        }
        else {
            out.push(MapAiTrigger { id, ..MapAiTrigger::default() });
        }
    }
    out
}

/// 按列解析 AITrigger CSV（短行缺列用默认，避免 Serde 把空串当成必填 i32 失败）。
fn parse_ai_trigger_csv_line(key: &str, value: &str) -> Option<MapAiTrigger> {
    let row = parse_westwood_csv_line(value);
    let col = |i: usize| row.fields.get(i).map(|f| f.as_str().trim()).unwrap_or("");
    let name = col(0);
    if name.is_empty() {
        return None;
    }
    let (compare_amount, compare_op) = decode_comparator(col(6));
    let weight = col(7).parse::<u32>().unwrap_or(ra_types::DEFAULT_AI_TRIGGER_WEIGHT).max(1);
    Some(MapAiTrigger {
        id: if key.is_empty() { AiTriggerName::parse(name) } else { AiTriggerName::parse(key) },
        name: name.to_string(),
        team: TeamTypeName::parse(col(1)),
        owner_house: HouseName::parse(col(2)),
        tech_level: col(3).parse().unwrap_or(0),
        condition: AiTriggerConditionKind::from_i32(col(4).parse().unwrap_or(-1)),
        condition_object: TechnoName::parse(col(5)),
        compare_amount,
        compare_op,
        // 原版长行：…, StartWeight, MinWeight, MaxWeight, IsForSkirmish, unused, side, base_defense, team2, Easy, Normal, Hard
        for_skirmish: parse_flag_default_true(col(10)),
        enabled_easy: parse_flag_default_true(col(15)),
        enabled_normal: parse_flag_default_true(col(16)),
        enabled_hard: parse_flag_default_true(col(17)),
        weight,
        team2: TeamTypeName::parse(col(14)),
    })
}

fn parse_flag_default_true(raw: &str) -> bool {
    if raw.is_empty() {
        return true;
    }
    raw.parse::<i32>().map(|v| v != 0).unwrap_or(true)
}

fn first_team(primary: TeamTypeName, fallback: TeamTypeName) -> TeamTypeName {
    if !primary.is_empty() { primary } else { fallback }
}

/// 解码 Comparator：完整 64 位十六进制块，或测试用短整数门槛。
pub fn decode_comparator(raw: &str) -> (i32, AiTriggerCompareOp) {
    let s = raw.trim();
    if s.is_empty() {
        return (0, AiTriggerCompareOp::GreaterEqual);
    }
    if s.len() >= 16 && s.chars().all(|c| c.is_ascii_hexdigit()) {
        let amount = parse_le_hex_u32(&s[0..8]) as i32;
        let op = AiTriggerCompareOp::from_u32(parse_le_hex_u32(&s[8..16]));
        return (amount, op);
    }
    if let Ok(n) = s.parse::<i32>() {
        // 短写：把数值当门槛，运算符默认 `>=`（常见「敌方至少拥有 N 座」）。
        return (n, AiTriggerCompareOp::GreaterEqual);
    }
    (0, AiTriggerCompareOp::GreaterEqual)
}

fn parse_le_hex_u32(octet: &str) -> u32 {
    // 8 个十六进制字符，按小端字节文本：`03000000` → 3。
    let mut bytes = [0u8; 4];
    for (i, chunk) in octet.as_bytes().chunks(2).take(4).enumerate() {
        let a = hex_val(chunk[0]);
        let b = hex_val(chunk.get(1).copied().unwrap_or(b'0'));
        bytes[i] = (a << 4) | b;
    }
    u32::from_le_bytes(bytes)
}

fn hex_val(c: u8) -> u8 {
    match c {
        b'0'..=b'9' => c - b'0',
        b'a'..=b'f' => c - b'a' + 10,
        b'A'..=b'F' => c - b'A' + 10,
        _ => 0,
    }
}
