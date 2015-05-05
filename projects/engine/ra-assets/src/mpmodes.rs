//! 多人 / 遭遇战模式表（`mpmodes.ini` / `mpmodesmd.ini`）。
//!
//! 行格式：`modeID=显示名CSF, 提示CSF, 规则覆盖INI, 地图过滤标签, 是否允许随机图`。

use ra_types::{RaError, RaResult};

use crate::IniDocument;

/// 一条可选多人模式。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MpMode {
    /// 模式编号（INI 键）。
    pub id: u32,
    /// 分类节名（如 `Battle` / `FreeForAll`）。
    pub category: String,
    /// 列表显示名 CSF 键（如 `GUI:Battle`）。
    pub name_csf: String,
    /// 状态栏提示 CSF 键（如 `STT:ModeBattle`）。
    pub tooltip_csf: String,
    /// 规则覆盖 INI 文件名（如 `MPBattle.ini`）。
    pub rules_override: String,
    /// 地图 `GameModes` 过滤标签（如 `standard`）。
    pub map_filter: String,
    /// 是否允许随机图。
    pub random_maps_allowed: bool,
}

impl MpMode {
    /// 离线遭遇战选图页默认是否展示该模式。
    ///
    /// 以「允许随机图」为数据门禁：原版表里对应「作战」与「自由交战」。
    pub fn visible_in_offline_skirmish(&self) -> bool {
        self.random_maps_allowed
    }
}

/// 从 `mpmodes.ini`（或 `mpmodesmd.ini`）字节解析模式列表，按 `id` 升序。
pub fn parse_mpmodes(bytes: &[u8]) -> RaResult<Vec<MpMode>> {
    let doc = IniDocument::parse(bytes)?;
    let mut modes = Vec::new();
    for section in &doc.sections {
        if section.name_key.is_empty() {
            continue;
        }
        for (key, value) in section.pairs() {
            let Ok(id) = key.trim().parse::<u32>() else {
                continue;
            };
            let mode = parse_mode_row(id, &section.name_raw, value).map_err(|e| {
                RaError::Parse(format!("mpmodes [{}] {}={}: {e}", section.name_raw, key, value.trim()))
            })?;
            modes.push(mode);
        }
    }
    modes.sort_by_key(|m| m.id);
    Ok(modes)
}

fn parse_mode_row(id: u32, category: &str, value: &str) -> Result<MpMode, String> {
    // 按逗号切分后 trim；保留中间空段以便发现缺列。
    let fields: Vec<&str> = value.split(',').map(|s| s.trim()).collect();
    if fields.len() < 4 {
        return Err(format!("需要至少 4 个逗号分隔字段，实际 {}", fields.len()));
    }
    let random_maps_allowed = if fields.len() >= 5 {
        parse_ini_bool(fields[4])?
    } else {
        false
    };
    Ok(MpMode {
        id,
        category: category.to_string(),
        name_csf: fields[0].to_string(),
        tooltip_csf: fields[1].to_string(),
        rules_override: fields[2].to_string(),
        map_filter: fields[3].to_string(),
        random_maps_allowed,
    })
}

fn parse_ini_bool(raw: &str) -> Result<bool, String> {
    match raw.trim().to_ascii_lowercase().as_str() {
        "true" | "yes" | "1" => Ok(true),
        "false" | "no" | "0" => Ok(false),
        other => Err(format!("无法解析布尔值 `{other}`")),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE: &str = r#"
; comment
[Battle]
1=GUI:Battle, STT:ModeBattle, MPBattle.ini, standard, true

[ManBattle]
5=GUI:Megawealth, STT:ModeMegawealth, MPMW.ini, megawealth, false

[FreeForAll]
2=GUI:FreeForAll, STT:ModeFreeForAll, MPFreeForAll.ini, standard, true

[Unholy]
4=GUI:UnholyAlliance, STT:ModeUnholyAlliance, MPUnholy.ini, standard, false

[Cooperative]
3=GUI:Cooperative, STT:ModeCooperative, MPCoop.ini, cooperative, false
"#;

    #[test]
    fn parses_stock_roster_sorted_by_id() {
        let modes = parse_mpmodes(SAMPLE.as_bytes()).unwrap();
        assert_eq!(modes.len(), 5);
        assert_eq!(modes.iter().map(|m| m.id).collect::<Vec<_>>(), vec![1, 2, 3, 4, 5]);
        assert_eq!(modes[0].name_csf, "GUI:Battle");
        assert_eq!(modes[0].map_filter, "standard");
        assert!(modes[0].random_maps_allowed);
        assert_eq!(modes[1].name_csf, "GUI:FreeForAll");
        assert_eq!(modes[1].category, "FreeForAll");
        assert!(!modes[4].random_maps_allowed);
        assert_eq!(modes[4].map_filter, "megawealth");
    }

    #[test]
    fn offline_skirmish_visibility_matches_random_flag() {
        let modes = parse_mpmodes(SAMPLE.as_bytes()).unwrap();
        let visible: Vec<_> = modes.iter().filter(|m| m.visible_in_offline_skirmish()).map(|m| m.id).collect();
        assert_eq!(visible, vec![1, 2]);
    }

    #[test]
    fn rejects_short_row() {
        let err = parse_mpmodes(b"[Battle]\n1=GUI:Battle, STT:ModeBattle\n").unwrap_err();
        let msg = err.to_string();
        assert!(msg.contains("至少 4"), "{msg}");
    }
}
