//! Techno / 建筑 / 图标共用的 `Image=` 解析。
//!
//! 顺序与原版一致：`rules` `Image=` → `art` `Image=` → 类型 id。
//! 美军空指 `AMRADR` 仅在 rules 写 `Image=GAAIRC` 时，须靠本函数跳到目标 art 节。

use ra_assets::IniDocument;

/// 解析主体 / 几何 / 图标共用的 image 键（大写）。
pub fn resolve_techno_image_key(rules: Option<&IniDocument>, art: Option<&IniDocument>, type_id: &str) -> String {
    let type_id = type_id.trim();
    let from_rules = rules.and_then(|r| r.get(type_id, "Image")).map(str::trim).filter(|s| !s.is_empty());
    let from_art = art.and_then(|a| a.get(type_id, "Image")).map(str::trim).filter(|s| !s.is_empty());
    from_rules.or(from_art).map(|s| s.to_ascii_uppercase()).unwrap_or_else(|| type_id.to_ascii_uppercase())
}
