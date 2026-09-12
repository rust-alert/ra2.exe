//! 从 `rules.ini` 的 `[OverlayTypes]` 建立 id → 名称表，并标记可采矿格。

use serde::Deserialize;

use ra_types::{LandType, OverlayName, OverlayTypeRegistry};

use crate::{
    ini::{IniDocument, IniMergePolicy, LayeredIniView},
    rules::{color_schemes::ColorSchemes, house_remap::Hsv},
};

/// 解析 `[OverlayTypes]`：按节内条目顺序赋 id `0..n`，忽略键的数字字面量。
///
/// 可采判定优先读类型节 `Tiberium=yes` / `SpawnsTiberium=yes` / `Land=`，
/// 再回退类型名前缀 `TIB*` / `GEM*`。
pub fn overlay_types_from_rules(rules: &IniDocument) -> OverlayTypeRegistry {
    let policy = IniMergePolicy::last_wins();
    let docs = std::slice::from_ref(rules);
    overlay_types_from_layered(LayeredIniView::new(docs, &policy))
}

/// 从层叠 rules 视图解析 `[OverlayTypes]`。
pub fn overlay_types_from_layered(view: LayeredIniView<'_>) -> OverlayTypeRegistry {
    let Some(section) = view.section("OverlayTypes")
    else {
        return OverlayTypeRegistry::default();
    };
    let mut names = Vec::new();
    let mut harvestable = Vec::new();
    let mut land_pass_override = Vec::new();
    for key in section.keys() {
        let Some(value) = section.get(key)
        else {
            continue;
        };
        let name = OverlayName::parse(value.trimmed().raw);
        if name.is_empty() {
            continue;
        }
        let name_up = name.as_str();
        let fields = overlay_type_fields(view, name_up);
        let can_harvest = overlay_type_is_harvestable(&fields, name_up);
        let pass_override = overlay_land_pass_override(&fields);
        names.push(name);
        harvestable.push(can_harvest);
        land_pass_override.push(pass_override);
    }
    OverlayTypeRegistry::from_entries(names, harvestable, land_pass_override)
}

/// 类型名是否像矿/宝石（无规则节时的回退）。
pub fn harvestable_overlay_name(name: &str) -> bool {
    let upper = name.to_ascii_uppercase();
    (upper.starts_with("TIB") && !upper.starts_with("TIBTRE")) || upper.starts_with("GEM")
}

/// overlay 类型名对应的 `[Tiberiums]` 条目（矿→`Riparius`，宝石→`Cruentus`）。
pub fn tiberium_type_for_overlay(name: &str) -> Option<&'static str> {
    let upper = name.to_ascii_uppercase();
    if upper.starts_with("GEM") {
        Some("Cruentus")
    } else if upper.starts_with("TIB") && !upper.starts_with("TIBTRE") {
        Some("Riparius")
    } else {
        None
    }
}

/// 矿石 / 宝石呈现用 HSV：读 `[Tiberiums]` 的 `Color=`，再经 `[Colors]` 解析。
///
/// 零售 `NeonGreen=0,0,0` 是矿石哨兵（不可直接 remap），替换为 `Gold`。
pub fn tiberium_overlay_display_hsv(rules: &IniDocument, colors: &ColorSchemes, overlay_name: &str) -> Option<Hsv> {
    let policy = IniMergePolicy::last_wins();
    let docs = std::slice::from_ref(rules);
    tiberium_overlay_display_hsv_layered(LayeredIniView::new(docs, &policy), colors, overlay_name)
}

/// 层叠 rules 下的矿石 / 宝石呈现 HSV。
///
/// 优先用装载期 `ColorSchemes::tiberium_display_hsv`；未绑定再读层叠文档。
pub fn tiberium_overlay_display_hsv_layered(view: LayeredIniView<'_>, colors: &ColorSchemes, overlay_name: &str) -> Option<Hsv> {
    let tib_type = tiberium_type_for_overlay(overlay_name)?;
    if let Some(hsv) = colors.tiberium_display_hsv(tib_type) {
        return Some(hsv);
    }
    let scheme = view.get(tib_type, "Color")?.trimmed().raw;
    let hsv = colors.get(scheme)?;
    if hsv == (Hsv { h: 0, s: 0, v: 0 }) {
        return Some(colors.get("Gold").unwrap_or(Hsv { h: 41, s: 240, v: 230 }));
    }
    Some(hsv)
}

/// 仅用装载期绑定表解析 overlay 呈现 HSV（运行时路径，不持有 `IniDocument`）。
pub fn tiberium_overlay_display_hsv_bound(colors: &ColorSchemes, overlay_name: &str) -> Option<Hsv> {
    let tib_type = tiberium_type_for_overlay(overlay_name)?;
    colors.tiberium_display_hsv(tib_type)
}

/// overlay 类型节字段（一次 Serde）。
#[derive(Debug, Default, Deserialize)]
struct OverlayTypeSectionFields {
    #[serde(rename = "Tiberium")]
    tiberium: Option<bool>,
    #[serde(rename = "SpawnsTiberium")]
    spawns_tiberium: Option<bool>,
    #[serde(rename = "Land", default)]
    land: LandType,
    #[serde(rename = "NoUseTileLandType")]
    no_use_tile_land_type: Option<bool>,
}

fn overlay_type_fields(view: LayeredIniView<'_>, name: &str) -> OverlayTypeSectionFields {
    view.section(name)
        .and_then(|s| s.deserialize::<OverlayTypeSectionFields>().ok())
        .unwrap_or_default()
}

/// `NoUseTileLandType=yes` 时按 `Land=` 得到通行覆盖；否则不改 TMP 封格。
///
/// 通行粗判与 `ra-map` 的 `land_passable` 对齐：水 / 岩 / 墙不可走，缺键按 Clear（可走）。
fn overlay_land_pass_override(fields: &OverlayTypeSectionFields) -> Option<bool> {
    if !fields.no_use_tile_land_type.unwrap_or(false) {
        return None;
    }
    Some(ra_types::land_passable(fields.land))
}

fn overlay_type_is_harvestable(fields: &OverlayTypeSectionFields, name: &str) -> bool {
    if fields.tiberium.unwrap_or(false) || fields.spawns_tiberium.unwrap_or(false) {
        return true;
    }
    if fields.land == LandType::Tiberium {
        return true;
    }
    harvestable_overlay_name(name)
}
