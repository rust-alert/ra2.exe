//! 一次性从遗留 `RustAlert.toml` 迁移到 settings/state JSON。

use std::path::PathBuf;

use ra_types::PresentFeel;
use serde::Deserialize;

use crate::{
    ConfigDiagnostic, ConfigLayer, DesktopSettings, DesktopState, MergedConfig, SkirmishLobbyPrefs,
    paths::{legacy_rust_alert_toml_path, LEGACY_RUST_ALERT_TOML},
    store::PersistStore,
};

/// 遗留 TOML 迁移结果。
#[derive(Debug, Default)]
pub struct LegacyMigration {
    /// 已写入的 settings（若本次迁移了 settings）。
    pub settings: Option<DesktopSettings>,
    /// 已写入的 state（若本次迁移了 state）。
    pub state: Option<DesktopState>,
}

#[derive(Debug, Default, Deserialize)]
struct PresentSectionFile {
    #[serde(default)]
    present: PresentFeel,
}

#[derive(Debug, Default, Deserialize)]
struct SkirmishSectionFile {
    #[serde(default)]
    skirmish: SkirmishLobbyPrefs,
}

/// 若 settings/state 均缺失且 exe 旁有遗留 TOML，则解析一次并写入 JSON。
pub(crate) fn try_migrate_legacy_toml(store: &dyn PersistStore) -> Option<(LegacyMigration, Vec<ConfigDiagnostic>)> {
    let settings_missing = matches!(crate::store::read_settings_text(store), Ok(None));
    let state_missing = matches!(crate::store::read_state_text(store), Ok(None));
    if !settings_missing && !state_missing {
        return None;
    }

    let path = legacy_rust_alert_toml_path();
    if !path.is_file() {
        return None;
    }
    let text = match std::fs::read_to_string(&path) {
        Ok(t) => t,
        Err(e) => {
            return Some((
                LegacyMigration::default(),
                vec![ConfigDiagnostic {
                    source: path.display().to_string(),
                    message: format!("遗留 {LEGACY_RUST_ALERT_TOML} 读取失败: {e}"),
                }],
            ));
        }
    };
    Some(migrate_toml_text_to_store(&text, &path.display().to_string(), store, settings_missing, state_missing))
}

/// 将遗留 TOML 文本迁入 store（供测试与 [`try_migrate_legacy_toml`]）。
pub fn migrate_toml_text_to_store(
    text: &str,
    source_label: &str,
    store: &dyn PersistStore,
    migrate_settings: bool,
    migrate_state: bool,
) -> (LegacyMigration, Vec<ConfigDiagnostic>) {
    let mut diagnostics = Vec::new();
    let label = source_label.to_string();
    let (table, mut diags) = crate::parse_toml_document(text, &label);
    diagnostics.append(&mut diags);

    let mut migrated = LegacyMigration::default();
    if migrate_settings {
        let layers = [ConfigLayer { label: label.clone(), table }];
        let merged = MergedConfig::merge_layers(&layers);
        let mut settings = DesktopSettings::from_merged(&merged);
        match toml_edit::de::from_str::<PresentSectionFile>(text) {
            Ok(file) => settings.present = file.present.sanitized(),
            Err(e) => diagnostics.push(ConfigDiagnostic {
                source: label.clone(),
                message: format!("遗留 [present] 解析失败，已用默认: {e}"),
            }),
        }
        if let Err(e) = settings.persist_to(store) {
            diagnostics.push(ConfigDiagnostic { source: label.clone(), message: format!("迁移写入 settings.json 失败: {e}") });
        }
        else {
            diagnostics.push(ConfigDiagnostic {
                source: label.clone(),
                message: format!("已从遗留 {LEGACY_RUST_ALERT_TOML} 迁移 settings.json"),
            });
            migrated.settings = Some(settings);
        }
    }

    if migrate_state {
        let skirmish = match toml_edit::de::from_str::<SkirmishSectionFile>(text) {
            Ok(file) => file.skirmish.sanitized(),
            Err(e) => {
                diagnostics.push(ConfigDiagnostic {
                    source: label.clone(),
                    message: format!("遗留 [skirmish] 解析失败，已用默认: {e}"),
                });
                SkirmishLobbyPrefs::default()
            }
        };
        let state = DesktopState { skirmish };
        if let Err(e) = state.persist_to(store) {
            diagnostics.push(ConfigDiagnostic { source: label.clone(), message: format!("迁移写入 state.json 失败: {e}") });
        }
        else {
            diagnostics.push(ConfigDiagnostic {
                source: label,
                message: format!("已从遗留 {LEGACY_RUST_ALERT_TOML} 迁移 state.json"),
            });
            migrated.state = Some(state);
        }
    }

    (migrated, diagnostics)
}

/// 从遗留 TOML 文本解析 `ra2_dir` / `edition`（供向上搜索的测试辅助）。
pub fn parse_ra2_dir_from_toml_text(text: &str) -> Option<(PathBuf, Option<String>)> {
    let mut ra2_dir = None;
    let mut edition = None;
    for line in text.lines() {
        let line = line.trim();
        if line.starts_with('#') {
            continue;
        }
        if let Some(rest) = line.strip_prefix("ra2_dir") {
            let v = rest.trim().trim_start_matches('=').trim().trim_matches('"');
            if !v.is_empty() {
                ra2_dir = Some(PathBuf::from(v));
            }
        }
        if let Some(rest) = line.strip_prefix("edition") {
            let v = rest.trim().trim_start_matches('=').trim().trim_matches('"');
            if !v.is_empty() {
                edition = Some(v.to_string());
            }
        }
    }
    Some((ra2_dir?, edition))
}
