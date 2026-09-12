//! 战役表（`battle.ini` / `battlemd.ini`）。
//!
//! `[Battles]` 列出战役 id；各节含 `Scenario` / `Description` 等字段。

use ra_types::{CampaignName, RaResult, UiName};
use serde::Deserialize;

use crate::IniDocument;

/// 一条战役开局定义（选边入口对应的首关）。
#[derive(Debug, Clone, PartialEq, Eq)]
#[doc(hidden)]
pub struct BattleCampaign {
    /// 战役 id（装载期一次解码为大写；如 `ALL1` / `TUT1` / `SOV1`）。
    pub id: CampaignName,
    /// 首关地图文件名（INI 原文；读取时可再规范化大小写）。
    pub scenario: String,
    /// 描述 CSF 键（装载期一次解码为大写；如 `DESC:ALL1`）；可空。
    pub description_csf: UiName,
    /// 所需光盘编号（`-1` 表示任意）。
    pub cd: i32,
    /// 是否仅调试战役（`DebugOnly=yes`）。
    pub debug_only: bool,
}

#[derive(Debug, Default, Deserialize)]
struct BattleSectionFields {
    #[serde(rename = "Scenario")]
    scenario: Option<String>,
    #[serde(rename = "Description", default)]
    description: UiName,
    #[serde(rename = "CD")]
    cd: Option<i32>,
    #[serde(rename = "DebugOnly")]
    debug_only: Option<bool>,
}

/// 从 `battle.ini` 字节解析战役表。
///
/// 只收录 `[Battles]` 列出且存在对应节、且带非空 `Scenario` 的条目；顺序跟列表。
pub fn parse_battle_campaigns(bytes: &[u8]) -> RaResult<Vec<BattleCampaign>> {
    let doc = IniDocument::parse(bytes)?;
    let Some(list) = doc.section("Battles")
    else {
        return Ok(Vec::new());
    };
    let mut out = Vec::new();
    // 按节内出现顺序收集 id（键多为 1=TUT1 这类序号）。
    let mut ids: Vec<String> = Vec::new();
    for (_key, value) in list.pairs() {
        let id = value.trim();
        if id.is_empty() {
            continue;
        }
        ids.push(id.to_string());
    }
    for raw_id in ids {
        let id = CampaignName::parse(&raw_id);
        let Some(section) = doc.section(raw_id.as_str()).or_else(|| doc.section(id.as_str()))
        else {
            continue;
        };
        let fields = section.deserialize::<BattleSectionFields>().unwrap_or_default();
        let scenario = fields.scenario.unwrap_or_default().trim().to_string();
        if scenario.is_empty() {
            continue;
        }
        let description_csf = fields.description;
        let cd = fields.cd.unwrap_or(-1);
        let debug_only = fields.debug_only.unwrap_or(false);
        out.push(BattleCampaign { id, scenario, description_csf, cd, debug_only });
    }
    Ok(out)
}

/// 按战役 id（大小写不敏感）查找一条记录。
pub fn find_battle_campaign<'a>(campaigns: &'a [BattleCampaign], id: &str) -> Option<&'a BattleCampaign> {
    campaigns.iter().find(|c| c.id.eq_ignore_ascii_case(id))
}
