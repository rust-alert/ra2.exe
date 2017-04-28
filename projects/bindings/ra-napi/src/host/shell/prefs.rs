//! 遭遇战大厅偏好：从 `RustAlert.toml` `[skirmish]` 恢复 / 写回。

use ra_config::SkirmishLobbyPrefs;
use ra_layout::SKIRMISH_ROW_COUNT;

use super::Shell;

impl Shell {
    /// 启动时应用已加载的遭遇战偏好（地图 / 模式 / 国家名稍后在 ensure_* 里对齐可用表）。
    pub(super) fn apply_skirmish_prefs(&mut self, prefs: SkirmishLobbyPrefs) {
        let prefs = prefs.sanitized();
        self.skirmish_prefs = prefs.clone();
        self.skirmish.player_name = prefs.player_name;
        self.skirmish.difficulty = prefs.difficulty;
        self.skirmish.short_game = prefs.short_game;
        self.skirmish.mcv_repacks = prefs.mcv_repacks;
        self.skirmish.crates = prefs.crates;
        self.skirmish.superweapons = prefs.superweapons;
        self.skirmish.build_off_ally = prefs.build_off_ally;
        self.skirmish.game_speed = prefs.game_speed;
        self.skirmish.credits = prefs.credits;
        self.skirmish.tech_level = prefs.tech_level;
        self.skirmish.unit_count = prefs.unit_count;
        self.skirmish.preferred_map = prefs.preferred_map.clone();
        self.selected_map = prefs.preferred_map;
        self.selected_mode_id = prefs.mode_id;
        for (row, color) in prefs.row_colors.iter().enumerate().take(SKIRMISH_ROW_COUNT) {
            self.skirmish.set_row_color(row, usize::from(*color));
        }
        // 国家短名在 `ensure_lobby_sides` 注入 sides 表后再套用。
        if let Some(local) = prefs.row_countries.first() {
            self.skirmish.side = local.clone();
        }
    }

    /// 按持久化国家名套用各行（须已有 `sides` 表）。
    pub(super) fn apply_skirmish_country_prefs(&mut self) {
        if self.skirmish.sides.is_empty() {
            return;
        }
        for (row, name) in self.skirmish_prefs.row_countries.iter().enumerate().take(SKIRMISH_ROW_COUNT) {
            self.skirmish.set_row_side_by_name(row, name);
        }
    }

    /// 从当前大厅状态抓取可持久化快照。
    pub(super) fn capture_skirmish_prefs(&self) -> SkirmishLobbyPrefs {
        let mut row_countries = Vec::with_capacity(SKIRMISH_ROW_COUNT);
        for row in 0..SKIRMISH_ROW_COUNT {
            let name = self.skirmish.row_side(row);
            if name.is_empty() {
                break;
            }
            row_countries.push(name.to_string());
        }
        SkirmishLobbyPrefs {
            preferred_map: self.selected_map.clone().or_else(|| self.skirmish.preferred_map.clone()),
            mode_id: self.selected_mode_id,
            player_name: self.skirmish.player_name.clone(),
            row_countries,
            row_colors: self.skirmish.row_colors.to_vec(),
            difficulty: self.skirmish.difficulty.clone(),
            short_game: self.skirmish.short_game,
            mcv_repacks: self.skirmish.mcv_repacks,
            crates: self.skirmish.crates,
            superweapons: self.skirmish.superweapons,
            build_off_ally: self.skirmish.build_off_ally,
            game_speed: self.skirmish.game_speed,
            credits: self.skirmish.credits,
            tech_level: self.skirmish.tech_level,
            unit_count: self.skirmish.unit_count,
        }
        .sanitized()
    }

    /// 写回 `RustAlert.toml` `[skirmish]`（失败只记日志）。
    pub(super) fn persist_skirmish_prefs(&mut self) {
        let prefs = self.capture_skirmish_prefs();
        self.skirmish_prefs = prefs.clone();
        if let Err(e) = ra_config::DesktopSettings::persist_skirmish_prefs(&prefs) {
            tracing::warn!(error = %e, "遭遇战偏好写回失败");
        }
    }
}
