//! 多人 / 遭遇战模式表（`mpmodes.ini` / `mpmodesmd.ini`）。
//!
//! 行格式：`modeID=显示名CSF, 提示CSF, 规则覆盖INI, 地图过滤标签, 是否允许随机图`。

use ra_types::{RaError, RaResult};
use serde::Deserialize;

use crate::{IniDocument, from_row, parse_numbered_key};

/// 一条可选多人模式。
#[derive(Debug, Clone, PartialEq, Eq)]
#[doc(hidden)]
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
            let Some(id) = parse_numbered_key(key)
            else {
                continue;
            };
            let mode = parse_mode_row(id, &section.name_raw, value)
                .map_err(|e| RaError::Parse(format!("mpmodes [{}] {}={}: {e}", section.name_raw, key, value.trim())))?;
            modes.push(mode);
        }
    }
    modes.sort_by_key(|m| m.id);
    Ok(modes)
}

#[derive(Debug, Deserialize)]
struct MpModeCsvRow {
    name_csf: String,
    tooltip_csf: String,
    rules_override: String,
    map_filter: String,
    #[serde(default)]
    random_maps_allowed: bool,
}

#[doc(hidden)]
pub fn parse_mode_row(id: u32, category: &str, value: &str) -> Result<MpMode, String> {
    // 按逗号切分后 trim；保留中间空段以便发现缺列。
    let field_count = value.split(',').count();
    if field_count < 4 {
        return Err(format!("需要至少 4 个逗号分隔字段，实际 {field_count}"));
    }
    let row: MpModeCsvRow = from_row(value).map_err(|e| e.to_string())?;
    Ok(MpMode {
        id,
        category: category.to_string(),
        name_csf: row.name_csf,
        tooltip_csf: row.tooltip_csf,
        rules_override: row.rules_override,
        map_filter: row.map_filter,
        random_maps_allowed: row.random_maps_allowed,
    })
}

#[doc(hidden)]
pub fn parse_ini_bool(raw: &str) -> Result<bool, String> {
    match raw.trim().to_ascii_lowercase().as_str() {
        "true" | "yes" | "1" => Ok(true),
        "false" | "no" | "0" => Ok(false),
        other => Err(format!("无法解析布尔值 `{other}`")),
    }
}
