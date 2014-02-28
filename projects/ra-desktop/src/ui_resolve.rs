//! 页面资源引用的可读性探测（不解码、不绘制）。
//!
//! 只回答「槽位里写下的文件名在当前挂载源里能否读到字节」。
//! 可读 ≠ 已进入 GPU UI pass，更 ≠ Pre-Alpha 视觉交付。

use ra_types::AssetSource;

use crate::ui_page::{UiAssetRef, UiPageResources};

/// 单页资源引用探测结果。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PageResolveReport {
    /// 已填写的资源引用数（背景 + 面板 + 各按钮状态名）。
    pub named: usize,
    /// 其中在挂载源上可读的数量。
    pub readable: usize,
    /// 已命名但读不到的逻辑文件名。
    pub missing: Vec<String>,
}

impl PageResolveReport {
    /// 简短标题栏备注。
    pub fn banner_note(&self) -> String {
        if self.named == 0 {
            "UI 引用 0 项 · 槽位未填资源名".into()
        } else {
            format!(
                "UI 引用可读 {}/{} · 缺 {}",
                self.readable,
                self.named,
                self.missing.len()
            )
        }
    }

    /// 是否所有已填名均可读（仍不表示已绘制）。
    pub fn all_named_readable(&self) -> bool {
        self.named > 0 && self.missing.is_empty()
    }
}

fn bump_name(
    source: &impl AssetSource,
    name: &str,
    named: &mut usize,
    readable: &mut usize,
    missing: &mut Vec<String>,
) {
    *named += 1;
    if source.read(name).is_ok() {
        *readable += 1;
    } else if !missing.iter().any(|m| m == name) {
        missing.push(name.to_string());
    }
}

fn bump_asset(
    source: &impl AssetSource,
    asset: &UiAssetRef,
    named: &mut usize,
    readable: &mut usize,
    missing: &mut Vec<String>,
) {
    bump_name(source, &asset.name, named, readable, missing);
    if let Some(pal) = &asset.palette {
        bump_name(source, pal, named, readable, missing);
    }
}

/// 探测一页已声明资源名在 `source` 上是否可读。
pub fn resolve_page(source: &impl AssetSource, page: &UiPageResources) -> PageResolveReport {
    let mut named = 0usize;
    let mut readable = 0usize;
    let mut missing = Vec::new();

    if let Some(bg) = &page.background {
        bump_asset(source, bg, &mut named, &mut readable, &mut missing);
    }
    if let Some(pal) = &page.background_palette {
        bump_name(source, pal, &mut named, &mut readable, &mut missing);
    }
    for panel in &page.panels {
        bump_asset(source, panel, &mut named, &mut readable, &mut missing);
    }
    for btn in &page.buttons {
        for asset in [&btn.normal, &btn.hover, &btn.pressed, &btn.disabled, &btn.focused]
            .into_iter()
            .flatten()
        {
            bump_asset(source, asset, &mut named, &mut readable, &mut missing);
        }
    }
    for font in &page.fonts {
        bump_name(source, font, &mut named, &mut readable, &mut missing);
    }

    PageResolveReport {
        named,
        readable,
        missing,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        menu_action::MenuAction,
        screen::OriginalScreen,
        ui_page::{UiButtonResources, UiPageResources, page_resources_from_slots},
    };
    use ra_types::RaError;
    use std::collections::HashMap;

    struct MemSource(HashMap<String, Vec<u8>>);

    impl AssetSource for MemSource {
        fn read(&self, name: &str) -> Result<Vec<u8>, RaError> {
            self.0
                .get(&name.to_ascii_lowercase())
                .cloned()
                .ok_or_else(|| RaError::Msg(format!("missing {name}")))
        }
    }

    #[test]
    fn empty_slots_report_zero_named() {
        let page = page_resources_from_slots(OriginalScreen::MainMenu).unwrap();
        let src = MemSource(HashMap::new());
        let report = resolve_page(&src, &page);
        assert_eq!(report.named, 0);
        assert!(!report.all_named_readable());
        assert!(report.banner_note().contains("0"));
    }

    #[test]
    fn counts_readable_and_missing() {
        let page = UiPageResources {
            screen: OriginalScreen::MainMenu,
            background: Some(UiAssetRef::named("bg.shp")),
            background_palette: Some("bg.pal".into()),
            panels: Vec::new(),
            buttons: vec![UiButtonResources {
                entry_id: "single_player",
                action: MenuAction::OpenSinglePlayer,
                enabled: true,
                hit: (0.0, 0.0, 1.0, 1.0),
                normal: Some(UiAssetRef::named("btn.shp")),
                hover: Some(UiAssetRef::named("missing.shp")),
                pressed: None,
                disabled: None,
                focused: None,
            }],
            fonts: Vec::new(),
        };
        let mut map = HashMap::new();
        map.insert("bg.shp".into(), vec![1]);
        map.insert("bg.pal".into(), vec![1]);
        map.insert("btn.shp".into(), vec![1]);
        let report = resolve_page(&MemSource(map), &page);
        assert_eq!(report.named, 4);
        assert_eq!(report.readable, 3);
        assert_eq!(report.missing, vec!["missing.shp".to_string()]);
    }
}
