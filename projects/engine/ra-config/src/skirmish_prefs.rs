//! 遭遇战大厅上次选择（`RustAlert.toml` 的 `[skirmish]`）。
//!
//! 只存壳层可恢复的大厅选项；不解释 rules / 地图语义。

use serde::{Deserialize, Serialize};

use crate::ConfigDiagnostic;

/// 遭遇战大厅持久化偏好。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct SkirmishLobbyPrefs {
    /// 优选地图文件名。
    pub preferred_map: Option<String>,
    /// 多人模式 id（`mpmodes`）。
    pub mode_id: Option<u32>,
    /// 本地玩家名。
    pub player_name: String,
    /// 各行国家短名（相对 rules；比下标更稳）。
    pub row_countries: Vec<String>,
    /// 各行色块下标。
    pub row_colors: Vec<u8>,
    /// AI 难度标签：`Easy` / `Normal` / `Hard`。
    pub difficulty: String,
    /// 快速游戏。
    pub short_game: bool,
    /// 基地重新部署。
    pub mcv_repacks: bool,
    /// 升级工具箱。
    pub crates: bool,
    /// 超级武器。
    pub superweapons: bool,
    /// 于盟友建造场旁建设。
    pub build_off_ally: bool,
    /// 游戏速度 0..=6。
    pub game_speed: u8,
    /// 起始资金。
    pub credits: i32,
    /// 科技上限。
    pub tech_level: i32,
    /// 起始部队数。
    pub unit_count: i32,
}

impl Default for SkirmishLobbyPrefs {
    fn default() -> Self {
        Self {
            preferred_map: None,
            mode_id: None,
            player_name: "Player".to_string(),
            row_countries: Vec::new(),
            row_colors: Vec::new(),
            difficulty: "Normal".to_string(),
            short_game: true,
            mcv_repacks: true,
            crates: true,
            superweapons: true,
            build_off_ally: false,
            game_speed: 6,
            credits: 10_000,
            tech_level: 10,
            unit_count: 10,
        }
    }
}

impl SkirmishLobbyPrefs {
    /// 夹紧到可落盘范围。
    pub fn sanitized(mut self) -> Self {
        if self.player_name.trim().is_empty() {
            self.player_name = "Player".to_string();
        }
        if self.player_name.chars().count() > 12 {
            self.player_name = self.player_name.chars().take(12).collect();
        }
        match self.difficulty.as_str() {
            "Easy" | "Normal" | "Hard" => {}
            _ => self.difficulty = "Normal".to_string(),
        }
        self.game_speed = self.game_speed.min(6);
        self.credits = self.credits.clamp(0, 10_000);
        self.tech_level = self.tech_level.clamp(1, 10);
        self.unit_count = self.unit_count.clamp(0, 20);
        if let Some(map) = self.preferred_map.as_mut() {
            let trimmed = map.trim().to_string();
            if trimmed.is_empty() {
                self.preferred_map = None;
            }
            else {
                *map = trimmed;
            }
        }
        self.row_countries.retain(|s| !s.trim().is_empty());
        for c in &mut self.row_countries {
            *c = c.trim().to_string();
        }
        self.row_colors.truncate(8);
        self
    }
}

/// 仅反序列化文档中的 `[skirmish]`（其余根键忽略）。
#[derive(Debug, Default, Deserialize)]
struct SkirmishSectionFile {
    #[serde(default)]
    skirmish: SkirmishLobbyPrefs,
}

/// 从完整 TOML 文本读取 `[skirmish]`；缺失则默认，失败则诊断并回退默认。
pub fn skirmish_prefs_from_toml_text(text: &str, source_label: &str) -> (SkirmishLobbyPrefs, Vec<ConfigDiagnostic>) {
    match toml_edit::de::from_str::<SkirmishSectionFile>(text) {
        Ok(file) => (file.skirmish.sanitized(), Vec::new()),
        Err(e) => (
            SkirmishLobbyPrefs::default(),
            vec![ConfigDiagnostic {
                source: source_label.into(),
                message: format!("[skirmish] 解析失败，已用默认遭遇战偏好: {e}"),
            }],
        ),
    }
}
