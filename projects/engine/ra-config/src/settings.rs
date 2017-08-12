//! 启动配置（`settings.json`，不含遭遇战记忆）。

use std::path::PathBuf;

use ra_types::{DisplayMode, PresentFeel, VgaExpandMode};
use serde::{Deserialize, Serialize};

use crate::{
    ConfigDiagnostic, MergedConfig,
    paths::exe_dir,
    store::{self, PersistStore},
};

fn take_emulate_override_snapshot() -> Option<crate::EmulateOverride> {
    crate::EMULATE_OVERRIDE.lock().expect("emulate override lock").clone()
}

/// 落盘用 DTO（`display_mode` 等用字符串，避免改 `ra-types`）。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
struct SettingsFile {
    ra2_dir: Option<String>,
    edition: Option<String>,
    display_mode: Option<String>,
    music_volume: Option<f32>,
    sound_volume: Option<f32>,
    present: PresentFeel,
    load_min_secs: Option<f64>,
    shell_slide_gap_secs: Option<f64>,
    palette_vga_expand: Option<String>,
    net_url: Option<String>,
    net_room: Option<String>,
}

impl Default for SettingsFile {
    fn default() -> Self {
        let d = DesktopSettings::default();
        Self {
            ra2_dir: Some(d.ra2_dir.to_string_lossy().replace('\\', "/")),
            edition: d.edition,
            display_mode: Some(d.display_mode.as_str().to_string()),
            music_volume: Some(d.music_volume),
            sound_volume: Some(d.sound_volume),
            present: d.present,
            load_min_secs: Some(d.load_min_secs),
            shell_slide_gap_secs: Some(d.shell_slide_gap_secs),
            palette_vga_expand: Some(d.palette_vga_expand.as_str().to_string()),
            net_url: d.net_url,
            net_room: d.net_room,
        }
    }
}

/// 桌面启动设置（config，不是 state）。
#[derive(Debug, Clone)]
pub struct DesktopSettings {
    /// 游戏安装目录（默认：可执行文件所在目录）。
    pub ra2_dir: PathBuf,
    /// 正式版本字符串（可选）。
    pub edition: Option<String>,
    /// 客户区显示分辨率档。
    pub display_mode: DisplayMode,
    /// 壳层 BGM 音量（0..1）。
    pub music_volume: f32,
    /// 壳层短音效音量（0..1）。
    pub sound_volume: f32,
    /// 壳层质感呈现。
    pub present: PresentFeel,
    /// 遭遇战装载页最短展示秒数。
    pub load_min_secs: f64,
    /// 壳层切页出去→进来停顿秒数。
    pub shell_slide_gap_secs: f64,
    /// VGA 调色板 6→8 bit 扩色。
    pub palette_vga_expand: VgaExpandMode,
    /// 预留战网地址。
    pub net_url: Option<String>,
    /// 预留房间名。
    pub net_room: Option<String>,
}

impl Default for DesktopSettings {
    fn default() -> Self {
        Self {
            ra2_dir: exe_dir(),
            edition: None,
            display_mode: DisplayMode::DEFAULT,
            music_volume: 0.4,
            sound_volume: 0.7,
            present: PresentFeel::DEFAULT,
            load_min_secs: 3.0,
            shell_slide_gap_secs: 0.2,
            palette_vga_expand: VgaExpandMode::Full,
            net_url: None,
            net_room: None,
        }
    }
}

impl DesktopSettings {
    /// 从合并扁平表填充（测试）。
    pub fn from_merged(merged: &MergedConfig) -> Self {
        let mut s = Self::default();
        if let Some(v) = merged.get("ra2_dir").or_else(|| merged.get("game_dir")) {
            s.ra2_dir = PathBuf::from(v);
        }
        if let Some(v) = merged.get("edition").filter(|v| !v.is_empty()) {
            s.edition = Some(v.to_string());
        }
        if let Some(v) = merged.get("display_mode").or_else(|| merged.get("resolution")).filter(|v| !v.is_empty()) {
            if let Ok(mode) = DisplayMode::parse(v) {
                s.display_mode = mode;
            }
        }
        if let Some(v) = merged.get("music_volume").or_else(|| merged.get("score_volume")).and_then(crate::parse_unit_volume) {
            s.music_volume = v;
        }
        if let Some(v) = merged.get("sound_volume").or_else(|| merged.get("sfx_volume")).and_then(crate::parse_unit_volume) {
            s.sound_volume = v;
        }
        if let Some(v) = merged.get("load_min_secs").and_then(|raw| raw.trim().parse::<f64>().ok()).filter(|v| v.is_finite()) {
            s.load_min_secs = v.max(0.0);
        }
        if let Some(v) = merged.get("shell_slide_gap_secs").and_then(|raw| raw.trim().parse::<f64>().ok()).filter(|v| v.is_finite()) {
            s.shell_slide_gap_secs = v.max(0.0);
        }
        if let Some(v) = merged.get("palette_vga_expand").and_then(VgaExpandMode::parse) {
            s.palette_vga_expand = v;
        }
        if let Some(v) = merged.get("net_url").or_else(|| merged.get("battlenet_url")).filter(|v| !v.is_empty()) {
            s.net_url = Some(v.to_string());
        }
        if let Some(v) = merged.get("net_room").or_else(|| merged.get("room")).filter(|v| !v.is_empty()) {
            s.net_room = Some(v.to_string());
        }
        s
    }

    fn from_file(file: SettingsFile) -> Self {
        let mut s = Self::default();
        if let Some(v) = file.ra2_dir.filter(|v| !v.trim().is_empty()) {
            s.ra2_dir = PathBuf::from(v);
        }
        if let Some(v) = file.edition.filter(|v| !v.trim().is_empty()) {
            s.edition = Some(v);
        }
        if let Some(v) = file.display_mode.as_deref() {
            if let Ok(mode) = DisplayMode::parse(v) {
                s.display_mode = mode;
            }
        }
        if let Some(v) = file.music_volume.filter(|v| v.is_finite()) {
            s.music_volume = v.clamp(0.0, 1.0);
        }
        if let Some(v) = file.sound_volume.filter(|v| v.is_finite()) {
            s.sound_volume = v.clamp(0.0, 1.0);
        }
        s.present = file.present.sanitized();
        if let Some(v) = file.load_min_secs.filter(|v| v.is_finite()) {
            s.load_min_secs = v.max(0.0);
        }
        if let Some(v) = file.shell_slide_gap_secs.filter(|v| v.is_finite()) {
            s.shell_slide_gap_secs = v.max(0.0);
        }
        if let Some(v) = file.palette_vga_expand.as_deref().and_then(VgaExpandMode::parse) {
            s.palette_vga_expand = v;
        }
        s.net_url = file.net_url.filter(|v| !v.trim().is_empty());
        s.net_room = file.net_room.filter(|v| !v.trim().is_empty());
        s
    }

    fn to_file(&self) -> SettingsFile {
        SettingsFile {
            ra2_dir: Some(self.ra2_dir.to_string_lossy().replace('\\', "/")),
            edition: self.edition.clone(),
            display_mode: Some(self.display_mode.as_str().to_string()),
            music_volume: Some(self.music_volume.clamp(0.0, 1.0)),
            sound_volume: Some(self.sound_volume.clamp(0.0, 1.0)),
            present: self.present.sanitized(),
            load_min_secs: Some(self.load_min_secs.max(0.0)),
            shell_slide_gap_secs: Some(self.shell_slide_gap_secs.max(0.0)),
            palette_vga_expand: Some(self.palette_vga_expand.as_str().to_string()),
            net_url: self.net_url.clone(),
            net_room: self.net_room.clone(),
        }
    }

    /// 序列化为 pretty JSON。
    pub fn to_json_text(&self) -> Result<String, String> {
        serde_json::to_string_pretty(&self.to_file()).map_err(|e| format!("序列化 settings 失败: {e}"))
    }

    /// 从 JSON 文本解析。
    pub fn from_json_text(text: &str, source_label: &str) -> (Self, Vec<ConfigDiagnostic>) {
        match serde_json::from_str::<SettingsFile>(text) {
            Ok(file) => (Self::from_file(file), Vec::new()),
            Err(e) => (
                Self::default(),
                vec![ConfigDiagnostic { source: source_label.into(), message: format!("settings.json 解析失败，已用默认: {e}") }],
            ),
        }
    }

    fn apply_emulate_override(&mut self, diagnostics: &mut Vec<ConfigDiagnostic>) {
        if let Some(over) = take_emulate_override_snapshot() {
            self.ra2_dir = over.ra2_dir;
            if over.edition.is_some() {
                self.edition = over.edition;
            }
            diagnostics.push(ConfigDiagnostic {
                source: "emulate-override".into(),
                message: format!("CLI/N-API 覆盖 ra2_dir={}", self.ra2_dir.display()),
            });
        }
    }

    /// 指定 store 加载（不套用一次性 emulate override，便于单测）。
    pub fn load_or_default_from(store: &dyn PersistStore) -> (Self, Vec<ConfigDiagnostic>) {
        Self::load_or_default_from_with_override(store, false)
    }

    /// 加载 settings：JSON ← 默认；可选套启动覆盖。
    pub fn load_or_default() -> (Self, Vec<ConfigDiagnostic>) {
        Self::load_or_default_from_with_override(store::default_store().as_ref(), true)
    }

    fn load_or_default_from_with_override(store: &dyn PersistStore, apply_override: bool) -> (Self, Vec<ConfigDiagnostic>) {
        let mut diagnostics = Vec::new();
        let mut settings = match store::read_settings_text(store) {
            Ok(Some(text)) => {
                let (s, mut diags) = Self::from_json_text(&text, "settings.json");
                diagnostics.append(&mut diags);
                s
            }
            Ok(None) => Self::default(),
            Err(e) => {
                diagnostics.push(ConfigDiagnostic { source: "settings.json".into(), message: e });
                Self::default()
            }
        };
        if apply_override {
            settings.apply_emulate_override(&mut diagnostics);
        }
        (settings, diagnostics)
    }

    /// 整份写回 settings.json。
    pub fn persist(&self) -> Result<(), String> {
        self.persist_to(store::default_store().as_ref())
    }

    /// 指定 store 写回。
    pub fn persist_to(&self, store: &dyn PersistStore) -> Result<(), String> {
        store::write_settings_text(store, &self.to_json_text()?)
    }

    /// 读盘（或默认）后改字段再写回；不套用一次性 emulate override。
    fn load_mutate_save(mutator: impl FnOnce(&mut Self)) -> Result<(), String> {
        let store = store::default_store();
        let mut settings = match store::read_settings_text(store.as_ref())? {
            Some(text) => Self::from_json_text(&text, "settings.json").0,
            None => Self::default(),
        };
        mutator(&mut settings);
        settings.persist_to(store.as_ref())
    }

    /// 写回 `display_mode`。
    pub fn persist_display_mode(mode: DisplayMode) -> Result<(), String> {
        Self::load_mutate_save(|s| s.display_mode = mode)
    }

    /// 写回音量。
    pub fn persist_audio_volumes(music_volume: f32, sound_volume: f32) -> Result<(), String> {
        Self::load_mutate_save(|s| {
            s.music_volume = music_volume.clamp(0.0, 1.0);
            s.sound_volume = sound_volume.clamp(0.0, 1.0);
        })
    }

    /// 写回质感呈现。
    pub fn persist_present_feel(feel: PresentFeel) -> Result<(), String> {
        Self::load_mutate_save(|s| s.present = feel.sanitized())
    }
}
