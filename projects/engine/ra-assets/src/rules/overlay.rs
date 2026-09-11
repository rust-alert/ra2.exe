//! 从 `rules.ini` 的 `[OverlayTypes]` 建立 id → 名称表，并标记可采矿格。

use ra_types::OverlayTypeRegistry;

use crate::{
    ini::IniDocument,
    rules::{color_schemes::ColorSchemes, house_remap::Hsv},
};

/// 解析 `[OverlayTypes]`：按节内条目顺序赋 id `0..n`，忽略键的数字字面量。
///
/// 可采判定优先读类型节 `Tiberium=yes` / `SpawnsTiberium=yes` / `Land=`，
/// 再回退类型名前缀 `TIB*` / `GEM*`。
pub fn overlay_types_from_rules(rules: &IniDocument) -> OverlayTypeRegistry {
    let Some(section) = rules.section("OverlayTypes")
    else {
        return OverlayTypeRegistry::default();
    };
    let mut names = Vec::new();
    let mut harvestable = Vec::new();
    let mut land_pass_override = Vec::new();
    for (_key, value) in section.pairs() {
        let name = value.trim();
        if name.is_empty() {
            continue;
        }
        let name_up = name.to_ascii_uppercase();
        let can_harvest = overlay_type_is_harvestable(rules, &name_up);
        let pass_override = overlay_land_pass_override(rules, &name_up);
        names.push(name_up);
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
    }
    else if upper.starts_with("TIB") && !upper.starts_with("TIBTRE") {
        Some("Riparius")
    }
    else {
        None
    }
}

/// 矿石 / 宝石呈现用 HSV：读 `[Tiberiums]` 的 `Color=`，再经 `[Colors]` 解析。
///
/// 零售 `NeonGreen=0,0,0` 是矿石哨兵（不可直接 remap），替换为 `Gold`。
pub fn tiberium_overlay_display_hsv(rules: &IniDocument, colors: &ColorSchemes, overlay_name: &str) -> Option<Hsv> {
    let tib_type = tiberium_type_for_overlay(overlay_name)?;
    let scheme = rules.get(tib_type, "Color")?;
    let hsv = colors.get(scheme)?;
    if hsv == (Hsv { h: 0, s: 0, v: 0 }) {
        return Some(colors.get("Gold").unwrap_or(Hsv { h: 41, s: 240, v: 230 }));
    }
    Some(hsv)
}

/// `NoUseTileLandType=yes` 时按 `Land=` 得到通行覆盖；否则不改 TMP 封格。
///
/// 通行粗判与 `ra-map` 的 `land_passable` 对齐：水 / 岩 / 墙不可走，缺键按 Clear（可走）。
fn overlay_land_pass_override(rules: &IniDocument, name: &str) -> Option<bool> {
    let no_use = rules
        .get(name, "NoUseTileLandType")
        .is_some_and(|v| v.eq_ignore_ascii_case("yes") || v == "1");
    if !no_use {
        return None;
    }
    let land = rules.get(name, "Land").unwrap_or("Clear");
    Some(match land.trim().to_ascii_lowercase().as_str() {
        "water" | "rock" | "wall" => false,
        _ => true,
    })
}

#[doc(hidden)]
pub fn overlay_type_is_harvestable(rules: &IniDocument, name: &str) -> bool {
    if rules.get(name, "Tiberium").is_some_and(|v| matches!(v.trim().to_ascii_lowercase().as_str(), "yes" | "true" | "1")) {
        return true;
    }
    if rules.get(name, "SpawnsTiberium").is_some_and(|v| matches!(v.trim().to_ascii_lowercase().as_str(), "yes" | "true" | "1")) {
        return true;
    }
    if rules.get(name, "Land").is_some_and(|v| matches!(v.trim().to_ascii_lowercase().as_str(), "tiberium" | "ore" | "gems")) {
        return true;
    }
    harvestable_overlay_name(name)
}
