//! 战役表（`battle.ini` / `battlemd.ini`）。
//!
//! `[Battles]` 列出战役 id；各节含 `Scenario` / `Description` 等字段。

use ra_types::RaResult;

use crate::IniDocument;

/// 一条战役开局定义（选边入口对应的首关）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BattleCampaign {
    /// 战役 id（如 `ALL1` / `TUT1` / `SOV1`）。
    pub id: String,
    /// 首关地图文件名（INI 原文；读取时可再规范化大小写）。
    pub scenario: String,
    /// 描述 CSF 键（如 `DESC:ALL1`）；可空。
    pub description_csf: String,
    /// 所需光盘编号（`-1` 表示任意）。
    pub cd: i32,
    /// 是否仅调试战役（`DebugOnly=yes`）。
    pub debug_only: bool,
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
    for id in ids {
        let Some(section) = doc.section(&id)
        else {
            continue;
        };
        let scenario = section.get("Scenario").unwrap_or("").trim().to_string();
        if scenario.is_empty() {
            continue;
        }
        let description_csf = section.get("Description").unwrap_or("").trim().to_string();
        let cd = section
            .get("CD")
            .and_then(|s| s.trim().parse::<i32>().ok())
            .unwrap_or(-1);
        let debug_only = section
            .get("DebugOnly")
            .map(|s| matches!(s.trim().to_ascii_lowercase().as_str(), "yes" | "true" | "1"))
            .unwrap_or(false);
        out.push(BattleCampaign {
            id,
            scenario,
            description_csf,
            cd,
            debug_only,
        });
    }
    Ok(out)
}

/// 按战役 id（大小写不敏感）查找一条记录。
pub fn find_battle_campaign<'a>(campaigns: &'a [BattleCampaign], id: &str) -> Option<&'a BattleCampaign> {
    campaigns.iter().find(|c| c.id.eq_ignore_ascii_case(id))
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE: &str = r#"
[Battles]
1=TUT1
2=ALL1
3=SOV1
4=E31

[TUT1]
CD=0
Scenario=trn01t.MAP
Description=DESC:TUT1

[ALL1]
CD=0
Scenario=ALL01t.MAP
Description=DESC:ALL1

[SOV1]
CD=1
Scenario=SOV01t.MAP
Description=DESC:SOV1

[E31]
CD=-1
Scenario=E31.map
DebugOnly=yes
Description=DESC:E31
"#;

    #[test]
    fn parses_stock_side_campaigns_in_list_order() {
        let camps = parse_battle_campaigns(SAMPLE.as_bytes()).unwrap();
        assert_eq!(camps.len(), 4);
        assert_eq!(camps[0].id, "TUT1");
        assert_eq!(camps[0].scenario, "trn01t.MAP");
        assert_eq!(camps[1].id, "ALL1");
        assert_eq!(camps[2].id, "SOV1");
        assert!(camps[3].debug_only);
        assert_eq!(find_battle_campaign(&camps, "all1").unwrap().description_csf, "DESC:ALL1");
    }

    #[test]
    fn empty_without_battles_section() {
        assert!(parse_battle_campaigns(b"[ALL1]\nScenario=x.map\n").unwrap().is_empty());
    }
}
