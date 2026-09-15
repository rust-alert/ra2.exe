//! 遭遇战大厅用户状态（写入 `state.json`，不是 settings）。

use serde::{Deserialize, Serialize};

/// 遭遇战大厅上次选择。
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
    /// 各行队伍号（`0` = 无队）。
    #[serde(default)]
    pub row_teams: Vec<u8>,
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
            row_teams: Vec::new(),
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
        self.row_teams.truncate(8);
        for t in &mut self.row_teams {
            *t %= 8;
        }
        self
    }
}

/// 桌面 / Web 用户状态文档。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct DesktopState {
    /// 遭遇战大厅记忆。
    pub skirmish: SkirmishLobbyPrefs,
}

impl Default for DesktopState {
    fn default() -> Self {
        Self { skirmish: SkirmishLobbyPrefs::default() }
    }
}

impl DesktopState {
    /// 夹紧字段。
    pub fn sanitized(mut self) -> Self {
        self.skirmish = self.skirmish.sanitized();
        self
    }

    /// 从 JSON 文本解析；失败返回诊断与默认。
    pub fn from_json_text(text: &str, source_label: &str) -> (Self, Vec<crate::ConfigDiagnostic>) {
        match serde_json::from_str::<DesktopState>(text) {
            Ok(state) => (state.sanitized(), Vec::new()),
            Err(e) => (
                Self::default(),
                vec![crate::ConfigDiagnostic { source: source_label.into(), message: format!("state.json 解析失败，已用默认: {e}") }],
            ),
        }
    }

    /// 序列化为 pretty JSON。
    pub fn to_json_text(&self) -> Result<String, String> {
        serde_json::to_string_pretty(&self.clone().sanitized()).map_err(|e| format!("序列化 state 失败: {e}"))
    }

    /// 经默认 store 加载；缺失则默认。
    pub fn load_or_default() -> (Self, Vec<crate::ConfigDiagnostic>) {
        Self::load_or_default_from(crate::store::default_store().as_ref())
    }

    /// 指定 store 加载。
    pub fn load_or_default_from(store: &dyn crate::store::PersistStore) -> (Self, Vec<crate::ConfigDiagnostic>) {
        let mut diagnostics = Vec::new();
        match crate::store::read_state_text(store) {
            Ok(Some(text)) => {
                let (state, mut diags) = Self::from_json_text(&text, "state.json");
                diagnostics.append(&mut diags);
                return (state, diagnostics);
            }
            Ok(None) => {}
            Err(e) => diagnostics.push(crate::ConfigDiagnostic { source: "state.json".into(), message: e }),
        }

        (Self::default(), diagnostics)
    }

    /// 整份写回。
    pub fn persist(&self) -> Result<(), String> {
        self.persist_to(crate::store::default_store().as_ref())
    }

    /// 指定 store 写回。
    pub fn persist_to(&self, store: &dyn crate::store::PersistStore) -> Result<(), String> {
        crate::store::write_state_text(store, &self.to_json_text()?)
    }

    /// 只更新遭遇战大厅记忆并写回。
    pub fn persist_skirmish(prefs: &SkirmishLobbyPrefs) -> Result<(), String> {
        let store = crate::store::default_store();
        let mut state = Self::load_or_default_from(store.as_ref()).0;
        state.skirmish = prefs.clone().sanitized();
        state.persist_to(store.as_ref())
    }
}
