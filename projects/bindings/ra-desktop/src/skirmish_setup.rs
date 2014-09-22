//! 遭遇战大厅配置：对话框 `0x102` 选项 + 装载请求。
//!
//! 控件几何在 [`crate::ui_layout::skirmish_lobby_layout`]；本模块只持状态与命中。

use crate::ui_layout::{RectPx, SkirmishLobbyLayout, SKIRMISH_CHECK_H, SKIRMISH_CHECK_W};

/// 大厅可选阵营短名（需与地图实体 `owner` 对得上才会成为本地玩家）。
/// 旗标 PCX 取自 `local.mix` 已证实文件名。
pub const LOBBY_SIDES: &[&str] = &["Americans", "French", "Germans", "British", "Russians"];

/// 大厅可选难度标签（写入装载请求；引擎按 Easy/Normal/Hard 调节 AI 节奏）。
pub const LOBBY_DIFFICULTIES: &[&str] = &["Easy", "Normal", "Hard"];

/// 阵营 → 安装内旗标 PCX（`local.mix` 证据）。
pub fn side_flag_pcx(side: &str) -> &'static str {
    match side {
        "Americans" => "usai.pcx",
        "French" => "frai.pcx",
        "Germans" => "geri.pcx",
        "British" => "gbri.pcx",
        "Russians" => "rusi.pcx",
        _ => "usai.pcx",
    }
}

/// 勾选框种类（对齐 `0x102` 控件 id）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SkirmishCheckbox {
    /// `0x54E` 快速游戏。
    ShortGame,
    /// `0x693` 基地重新部署。
    McvRepacks,
    /// `0x696` 升级工具箱。
    Crates,
    /// `0x69A` 超级武器。
    SuperWeapons,
    /// `0x69D` 于盟友建造场旁建设。
    BuildOffAlly,
}

impl SkirmishCheckbox {
    const ALL: [Self; 5] = [
        Self::ShortGame,
        Self::McvRepacks,
        Self::Crates,
        Self::SuperWeapons,
        Self::BuildOffAlly,
    ];
}

/// 滑条种类（对齐 `0x102`）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SkirmishTrackbar {
    /// `0x529` 游戏速度（0..=6）。
    GameSpeed,
    /// `0x511` 资金（千为单位档，对应金额）。
    Credits,
    /// `0x50C` 部队数。
    UnitCount,
}

impl SkirmishTrackbar {
    /// 滑条最大值（含）。
    pub const fn max(self) -> i32 {
        match self {
            Self::GameSpeed => 6,
            // 零售 `[MultiplayerDialogSettings]` 常见 MaxMoney/1000；缺省按 10k→10。
            Self::Credits => 10,
            Self::UnitCount => 20,
        }
    }

    /// 步进。
    pub const fn step(self) -> i32 {
        1
    }
}

/// 遭遇战大厅左栏命中。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SkirmishLobbyHit {
    /// 切换勾选。
    Toggle(SkirmishCheckbox),
    /// 点在滑条上（开始拖或跳档）。
    Track(SkirmishTrackbar),
    /// 点本地国家下拉面 → 循环阵营。
    CycleSide,
}

/// 遭遇战装载请求（大厅选项的可序列化快照）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SkirmishBootRequest {
    /// 本地玩家名（零售默认 `Player`）。
    pub player_name: String,
    /// 优选地图文件名。
    pub preferred_map: Option<String>,
    /// 期望本地阵营（规则/地图 house 名）。
    pub side: String,
    /// 难度标签。
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
    /// 起始部队数。
    pub unit_count: i32,
    /// 正在拖动的滑条。
    pub dragging: Option<SkirmishTrackbar>,
}

impl SkirmishBootRequest {
    /// 默认：玩家名 `Player`、无指定图、盟军、普通难度；勾选对齐零售默认（盟友旁建造关）。
    pub fn default_lobby() -> Self {
        Self {
            player_name: "Player".to_string(),
            preferred_map: None,
            side: LOBBY_SIDES[0].to_string(),
            difficulty: LOBBY_DIFFICULTIES[1].to_string(),
            short_game: true,
            mcv_repacks: true,
            crates: true,
            superweapons: true,
            build_off_ally: false,
            game_speed: 6,
            credits: 10_000,
            unit_count: 10,
            dragging: None,
        }
    }

    /// 循环下一阵营。
    pub fn cycle_side(&mut self) {
        let i = LOBBY_SIDES.iter().position(|s| *s == self.side.as_str()).unwrap_or(0);
        self.side = LOBBY_SIDES[(i + 1) % LOBBY_SIDES.len()].to_string();
    }

    /// 循环下一难度。
    pub fn cycle_difficulty(&mut self) {
        let i = LOBBY_DIFFICULTIES.iter().position(|s| *s == self.difficulty.as_str()).unwrap_or(1);
        self.difficulty = LOBBY_DIFFICULTIES[(i + 1) % LOBBY_DIFFICULTIES.len()].to_string();
    }

    /// 装载笔记片段。
    pub fn note_fragment(&self) -> String {
        format!(
            "side={} diff={} map={} speed={} credits={} units={}",
            self.side,
            self.difficulty,
            self.preferred_map.as_deref().unwrap_or("(auto)"),
            self.game_speed,
            self.credits,
            self.unit_count
        )
    }

    fn checkbox_value(&self, id: SkirmishCheckbox) -> bool {
        match id {
            SkirmishCheckbox::ShortGame => self.short_game,
            SkirmishCheckbox::McvRepacks => self.mcv_repacks,
            SkirmishCheckbox::Crates => self.crates,
            SkirmishCheckbox::SuperWeapons => self.superweapons,
            SkirmishCheckbox::BuildOffAlly => self.build_off_ally,
        }
    }

    fn set_checkbox(&mut self, id: SkirmishCheckbox, value: bool) {
        match id {
            SkirmishCheckbox::ShortGame => self.short_game = value,
            SkirmishCheckbox::McvRepacks => self.mcv_repacks = value,
            SkirmishCheckbox::Crates => self.crates = value,
            SkirmishCheckbox::SuperWeapons => self.superweapons = value,
            SkirmishCheckbox::BuildOffAlly => self.build_off_ally = value,
        }
    }

    fn track_pos(&self, id: SkirmishTrackbar) -> i32 {
        match id {
            SkirmishTrackbar::GameSpeed => i32::from(self.game_speed),
            SkirmishTrackbar::Credits => (self.credits / 1000).clamp(0, SkirmishTrackbar::Credits.max()),
            SkirmishTrackbar::UnitCount => self.unit_count.clamp(0, SkirmishTrackbar::UnitCount.max()),
        }
    }

    fn set_track_pos(&mut self, id: SkirmishTrackbar, pos: i32) {
        let pos = pos.clamp(0, id.max());
        match id {
            SkirmishTrackbar::GameSpeed => self.game_speed = pos as u8,
            SkirmishTrackbar::Credits => self.credits = pos * 1000,
            SkirmishTrackbar::UnitCount => self.unit_count = pos,
        }
    }

    /// 按下：勾选切换、滑条拖动，或点国家面循环阵营。
    pub fn on_press(&mut self, layout: &SkirmishLobbyLayout, x: i32, y: i32) -> Option<SkirmishLobbyHit> {
        for (i, id) in SkirmishCheckbox::ALL.iter().enumerate() {
            let rect = layout.checkboxes[i];
            let icon = RectPx::new(rect.x, rect.y, SKIRMISH_CHECK_W, SKIRMISH_CHECK_H.min(rect.h.max(SKIRMISH_CHECK_H)));
            // 图标或整行标签区均可点（对齐零售勾选行为）。
            if icon.contains(x, y) || rect.contains(x, y) {
                self.set_checkbox(*id, !self.checkbox_value(*id));
                return Some(SkirmishLobbyHit::Toggle(*id));
            }
        }
        for id in [SkirmishTrackbar::GameSpeed, SkirmishTrackbar::Credits, SkirmishTrackbar::UnitCount] {
            let rect = track_rect(layout, id);
            if rect.contains(x, y) {
                self.dragging = Some(id);
                self.set_track_pos(id, track_pos_from_mouse(rect, x, id));
                return Some(SkirmishLobbyHit::Track(id));
            }
        }
        // 本地国家下拉面（行 0）：暂用点击循环，完整列表后续再接。
        if layout.side_faces[0].contains(x, y) {
            self.cycle_side();
            return Some(SkirmishLobbyHit::CycleSide);
        }
        None
    }

    /// 拖动滑条。
    pub fn on_drag(&mut self, layout: &SkirmishLobbyLayout, x: i32, _y: i32) -> bool {
        let Some(id) = self.dragging
        else {
            return false;
        };
        let rect = track_rect(layout, id);
        let next = track_pos_from_mouse(rect, x, id);
        if next == self.track_pos(id) {
            return true;
        }
        self.set_track_pos(id, next);
        true
    }

    /// 释放拖动。
    pub fn on_release(&mut self) {
        self.dragging = None;
    }
}

/// 光标下的悬停入口 id（供底栏 `STT:Skirmish*`；不改状态）。
pub fn hover_entry_at(layout: &SkirmishLobbyLayout, x: i32, y: i32) -> Option<&'static str> {
    if layout.player_name.contains(x, y) {
        return Some("player_name");
    }
    if layout.flags[0].contains(x, y) {
        return Some("flag");
    }
    if layout.side_faces[0].contains(x, y) {
        return Some("country");
    }
    if layout.color_faces[0].contains(x, y) {
        return Some("color");
    }
    for face in &layout.ai_faces {
        if face.contains(x, y) {
            return Some("ai");
        }
    }
    for (i, id) in SkirmishCheckbox::ALL.iter().enumerate() {
        let rect = layout.checkboxes[i];
        let icon = RectPx::new(rect.x, rect.y, SKIRMISH_CHECK_W, SKIRMISH_CHECK_H.min(rect.h.max(SKIRMISH_CHECK_H)));
        if icon.contains(x, y) || rect.contains(x, y) {
            return Some(match id {
                SkirmishCheckbox::ShortGame => "short_game",
                SkirmishCheckbox::McvRepacks => "mcv_repacks",
                SkirmishCheckbox::Crates => "crates",
                SkirmishCheckbox::SuperWeapons => "superweapons",
                SkirmishCheckbox::BuildOffAlly => "build_off_ally",
            });
        }
    }
    if layout.track_speed.contains(x, y) || layout.label_speed.contains(x, y) {
        return Some("speed");
    }
    if layout.track_credits.contains(x, y) || layout.label_credits.contains(x, y) {
        return Some("credits");
    }
    if layout.track_units.contains(x, y) || layout.label_units.contains(x, y) {
        return Some("units");
    }
    if layout.map_preview.contains(x, y) {
        return Some("map_preview");
    }
    if layout.game_type.contains(x, y) {
        return Some("game_type");
    }
    if layout.map_label.contains(x, y) {
        return Some("map_label");
    }
    None
}

fn track_rect(layout: &SkirmishLobbyLayout, id: SkirmishTrackbar) -> RectPx {
    match id {
        SkirmishTrackbar::GameSpeed => layout.track_speed,
        SkirmishTrackbar::Credits => layout.track_credits,
        SkirmishTrackbar::UnitCount => layout.track_units,
    }
}

fn track_pos_from_mouse(rect: RectPx, mouse_x: i32, id: SkirmishTrackbar) -> i32 {
    let max = id.max().max(1);
    let travel = (rect.w - 12).max(1);
    let rel = (mouse_x - rect.x - 6).clamp(0, travel);
    (rel * max + travel / 2) / travel
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ui_layout::skirmish_lobby_layout;

    #[test]
    fn default_options_match_retail_defaults() {
        let s = SkirmishBootRequest::default_lobby();
        assert!(s.short_game && s.mcv_repacks && s.crates && s.superweapons);
        assert!(!s.build_off_ally);
        assert_eq!(s.game_speed, 6);
        assert_eq!(s.credits, 10_000);
        assert_eq!(s.unit_count, 10);
        assert_eq!(s.player_name, "Player");
    }

    #[test]
    fn checkbox_toggle_and_track_drag() {
        let layout = skirmish_lobby_layout(800, 600);
        let mut s = SkirmishBootRequest::default_lobby();
        let r = layout.checkboxes[0];
        assert_eq!(s.on_press(&layout, r.x + 2, r.y + 2), Some(SkirmishLobbyHit::Toggle(SkirmishCheckbox::ShortGame)));
        assert!(!s.short_game);
        let t = layout.track_speed;
        assert_eq!(s.on_press(&layout, t.x + 4, t.y + 4), Some(SkirmishLobbyHit::Track(SkirmishTrackbar::GameSpeed)));
        assert!(s.on_drag(&layout, t.x + t.w - 4, t.y + 4));
        assert_eq!(s.game_speed, 6);
        s.on_release();
        assert!(s.dragging.is_none());
    }

    #[test]
    fn side_face_click_cycles_side() {
        let layout = skirmish_lobby_layout(800, 600);
        let mut s = SkirmishBootRequest::default_lobby();
        assert_eq!(s.side, "Americans");
        let face = layout.side_faces[0];
        assert_eq!(s.on_press(&layout, face.x + 2, face.y + 2), Some(SkirmishLobbyHit::CycleSide));
        assert_eq!(s.side, "French");
    }

    #[test]
    fn americans_flag_pcx() {
        assert_eq!(side_flag_pcx("Americans"), "usai.pcx");
        assert_eq!(side_flag_pcx("Russians"), "rusi.pcx");
    }

    #[test]
    fn hover_entry_reports_checkbox_and_country() {
        let layout = skirmish_lobby_layout(800, 600);
        let r = layout.checkboxes[0];
        assert_eq!(hover_entry_at(&layout, r.x + 2, r.y + 2), Some("short_game"));
        let face = layout.side_faces[0];
        assert_eq!(hover_entry_at(&layout, face.x + 2, face.y + 2), Some("country"));
        assert_eq!(
            hover_entry_at(&layout, layout.map_preview.x + 2, layout.map_preview.y + 2),
            Some("map_preview")
        );
    }
}
