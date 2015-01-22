//! 遭遇战大厅配置：对话框 `0x102` 选项 + 装载请求。
//!
//! 控件几何在 [`ra_layout::ui_layout::skirmish_lobby_layout`]；本模块只持状态与命中。

use ra_layout::ui_layout::{RectPx, SKIRMISH_CHECK_H, SKIRMISH_CHECK_W, SKIRMISH_COMBO_FACE_H, SKIRMISH_ROW_COUNT, SkirmishLobbyLayout};

/// 大厅可选阵营短名（写入装载请求；遭遇战会登记进玩家表，不要求地图实体已有同名 owner）。
/// 旗标 PCX 取自 `local.mix` 已证实文件名。
pub const LOBBY_SIDES: &[&str] = &["Americans", "French", "Germans", "British", "Russians"];

/// 大厅可选难度标签（写入装载请求；引擎按 Easy/Normal/Hard 调节 AI 节奏）。
pub const LOBBY_DIFFICULTIES: &[&str] = &["Easy", "Normal", "Hard"];

/// 大厅可选玩家色块（RGB；点击颜色面循环）。
pub const LOBBY_COLORS: &[[u8; 3]] = &[
    [255, 214, 0],  // 金黄
    [200, 24, 24],  // 红
    [32, 72, 200],  // 蓝
    [0, 160, 0],    // 绿
    [220, 120, 16], // 橙
    [0, 180, 180],  // 青
    [140, 48, 180], // 紫
    [220, 80, 160], // 粉
];

/// 玩家名最大字符数（零售 Handle 常见上限）。
pub const PLAYER_NAME_MAX_CHARS: usize = 12;

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

/// 阵营 → 标准装载艺术图后缀（`ls800{suffix}.shp` / `ls640{suffix}.shp`）。
pub fn load_screen_art_suffix(side: &str) -> &'static str {
    match side {
        "Americans" => "ustates",
        "French" => "france",
        "Germans" => "germany",
        "British" => "ukingdom",
        "Russians" => "russia",
        "Alliance" | "Korea" | "Koreans" => "korea",
        "Confederation" | "Cuba" | "Cubans" => "cuba",
        "Arabs" | "Iraq" | "Iraqis" => "iraq",
        "Africans" | "Libya" | "Libyans" => "libya",
        "YuriCountry" | "Yuri" => "yuri",
        "Observer" | "Observers" => "obs",
        _ => "ustates",
    }
}

/// 阵营 → CSF `LOADBRIEF:*` / `LOADBRIEFSHORT:*` 后缀（与 `ra2.csf` 键一致）。
pub fn load_screen_brief_suffix(side: &str) -> &'static str {
    match side {
        "Americans" => "USA",
        "French" => "FRENCH",
        "Germans" => "GERMANS",
        "British" => "BRITISH",
        "Russians" => "RUSSIA",
        "Alliance" | "Korea" | "Koreans" => "KOREA",
        "Confederation" | "Cuba" | "Cubans" => "CUBA",
        "Arabs" | "Iraq" | "Iraqis" => "IRAQ",
        "Africans" | "Libya" | "Libyans" => "LYBIA",
        _ => "USA",
    }
}

/// 装载图调色板：原版 / 共和国之辉盘均为共享 `mpls.pal`（`cache.mix`），无国家专用 `mplsu` 等。
pub const LOAD_SCREEN_FALLBACK_PAL: &str = "mpls.pal";

/// 阵营 → 装载图优先调色板（原版链只有共享盘，恒为 [`LOAD_SCREEN_FALLBACK_PAL`]）。
pub fn load_screen_preferred_pal(_side: &str) -> &'static str {
    LOAD_SCREEN_FALLBACK_PAL
}

/// 进度条 SHP（帧 0；按进度横向裁剪填充）。
pub const LOAD_SCREEN_PROGRESS_SHP: &str = "progbarm.shp";

/// 按视口宽选 `ls800*` / `ls640*` 背景名。
pub fn load_screen_background_shp(side: &str, viewport_w: u32) -> String {
    let suffix = load_screen_art_suffix(side);
    let prefix = if viewport_w >= 800 { "ls800" } else { "ls640" };
    format!("{prefix}{suffix}.shp")
}

/// 选择可读的装载调色板（原版链：共享 `mpls.pal`）。
pub fn load_screen_palette(side: &str, pal_readable: impl Fn(&str) -> bool) -> &'static str {
    let preferred = load_screen_preferred_pal(side);
    if pal_readable(preferred) {
        preferred
    } else {
        // 缺盘时仍返回规范名，解码失败由壳层空页路径兜底。
        preferred
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
    const ALL: [Self; 5] = [Self::ShortGame, Self::McvRepacks, Self::Crates, Self::SuperWeapons, Self::BuildOffAlly];
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
    /// 打开 / 关闭国家下拉。
    ToggleCountryCombo,
    /// 在国家下拉里选中一项。
    PickCountry(usize),
    /// 打开 / 关闭颜色下拉。
    ToggleColorCombo,
    /// 在颜色下拉里选中一项。
    PickColor(usize),
    /// 打开 / 关闭 AI 难度下拉。
    ToggleAiCombo,
    /// 在 AI 难度下拉里选中一项。
    PickAi(usize),
    /// 聚焦玩家名编辑框。
    FocusName,
}

/// 展开中的下拉种类。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SkirmishComboKind {
    /// 本地国家。
    Country,
    /// 本地颜色。
    Color,
    /// AI 难度（行 0）。
    Ai,
}

/// 遭遇战装载请求（大厅选项的可序列化快照）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SkirmishBootRequest {
    /// 本地玩家名（零售默认 `Player`）。
    pub player_name: String,
    /// 优选地图文件名。
    pub preferred_map: Option<String>,
    /// 期望本地阵营（规则/地图 house 名；与 `row_sides[0]` 同步）。
    pub side: String,
    /// 各行国家下标（`LOBBY_SIDES`；行 0 本地，其后 AI）。
    pub row_sides: [u8; SKIRMISH_ROW_COUNT],
    /// 难度标签。
    pub difficulty: String,
    /// 本地玩家色块下标（`LOBBY_COLORS`；与 `row_colors[0]` 同步）。
    pub color_index: u8,
    /// 各行色块下标（`LOBBY_COLORS`）。
    pub row_colors: [u8; SKIRMISH_ROW_COUNT],
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
    /// 玩家名编辑框是否聚焦。
    pub player_name_editing: bool,
    /// 展开中的下拉（`None` 表示收起）。
    pub open_combo: Option<SkirmishComboKind>,
    /// 当前下拉所在玩家行（0 本地）。
    pub combo_row: usize,
}

impl SkirmishBootRequest {
    /// 默认：玩家名 `Player`、无指定图、盟军、普通难度；勾选对齐零售默认（盟友旁建造关）。
    pub fn default_lobby() -> Self {
        Self {
            player_name: "Player".to_string(),
            preferred_map: None,
            side: LOBBY_SIDES[0].to_string(),
            row_sides: default_row_sides(),
            difficulty: LOBBY_DIFFICULTIES[1].to_string(),
            color_index: 0,
            row_colors: default_row_colors(),
            short_game: true,
            mcv_repacks: true,
            crates: true,
            superweapons: true,
            build_off_ally: false,
            game_speed: 6,
            credits: 10_000,
            unit_count: 10,
            dragging: None,
            player_name_editing: false,
            open_combo: None,
            combo_row: 0,
        }
    }

    /// 循环下一阵营（仅本地行）。
    pub fn cycle_side(&mut self) {
        let i = self.row_side_index(0);
        self.set_row_side(0, (i + 1) % LOBBY_SIDES.len());
    }

    /// 循环下一色块（仅本地行）。
    pub fn cycle_color(&mut self) {
        let i = self.row_color_index(0);
        self.set_row_color(0, (i + 1) % LOBBY_COLORS.len());
    }

    /// 当前色块 RGB（本地行）。
    pub fn color_rgb(&self) -> [u8; 3] {
        self.row_color_rgb(0)
    }

    /// 指定行国家短名。
    pub fn row_side(&self, row: usize) -> &'static str {
        LOBBY_SIDES[self.row_side_index(row)]
    }

    /// 指定行色块 RGB。
    pub fn row_color_rgb(&self, row: usize) -> [u8; 3] {
        LOBBY_COLORS[self.row_color_index(row)]
    }

    fn row_side_index(&self, row: usize) -> usize {
        (self.row_sides[row.min(SKIRMISH_ROW_COUNT - 1)] as usize) % LOBBY_SIDES.len()
    }

    fn row_color_index(&self, row: usize) -> usize {
        (self.row_colors[row.min(SKIRMISH_ROW_COUNT - 1)] as usize) % LOBBY_COLORS.len()
    }

    /// 结束玩家名编辑。
    pub fn end_name_edit(&mut self) {
        self.player_name_editing = false;
        if self.player_name.trim().is_empty() {
            self.player_name = "Player".to_string();
        }
    }

    /// 收起下拉。
    pub fn close_combo(&mut self) {
        self.open_combo = None;
        self.combo_row = 0;
    }

    /// 国家下拉列表矩形（紧贴指定行国家面下方）。
    pub fn country_list_rect(layout: &SkirmishLobbyLayout, row: usize) -> RectPx {
        let face = layout.side_faces[row.min(layout.side_faces.len().saturating_sub(1))];
        RectPx::new(face.x, face.y + face.h, face.w, SKIRMISH_COMBO_FACE_H * LOBBY_SIDES.len() as i32)
    }

    /// 颜色下拉列表矩形（紧贴指定行颜色面下方）。
    pub fn color_list_rect(layout: &SkirmishLobbyLayout, row: usize) -> RectPx {
        let face = layout.color_faces[row.min(layout.color_faces.len().saturating_sub(1))];
        RectPx::new(face.x, face.y + face.h, face.w.max(28), SKIRMISH_COMBO_FACE_H * LOBBY_COLORS.len() as i32)
    }

    /// AI 难度下拉列表矩形（紧贴行 0 AI 面下方）。
    pub fn ai_list_rect(layout: &SkirmishLobbyLayout) -> RectPx {
        let face = layout.ai_faces[0];
        RectPx::new(face.x, face.y + face.h, face.w, SKIRMISH_COMBO_FACE_H * LOBBY_DIFFICULTIES.len() as i32)
    }

    /// 设置指定行阵营为 `LOBBY_SIDES[index]`。
    pub fn set_row_side(&mut self, row: usize, index: usize) {
        if row >= SKIRMISH_ROW_COUNT {
            return;
        }
        if let Some(side) = LOBBY_SIDES.get(index) {
            self.row_sides[row] = index as u8;
            if row == 0 {
                self.side = (*side).to_string();
            }
        }
    }

    /// 设置当前下拉行的阵营。
    pub fn set_side_index(&mut self, index: usize) {
        self.set_row_side(self.combo_row, index);
    }

    /// 设置指定行色块为 `LOBBY_COLORS[index]`。
    pub fn set_row_color(&mut self, row: usize, index: usize) {
        if row >= SKIRMISH_ROW_COUNT || index >= LOBBY_COLORS.len() {
            return;
        }
        self.row_colors[row] = index as u8;
        if row == 0 {
            self.color_index = index as u8;
        }
    }

    /// 设置当前下拉行的色块。
    pub fn set_color_index(&mut self, index: usize) {
        self.set_row_color(self.combo_row, index);
    }

    /// 设置 AI 难度为 `LOBBY_DIFFICULTIES[i]`。
    pub fn set_difficulty_index(&mut self, index: usize) {
        if let Some(diff) = LOBBY_DIFFICULTIES.get(index) {
            self.difficulty = (*diff).to_string();
        }
    }

    /// AI 难度对应 CSF 标签键。
    pub fn ai_difficulty_csf_key(difficulty: &str) -> &'static str {
        match difficulty {
            "Easy" => "GUI:AIEasy",
            "Hard" => "GUI:AIHard",
            _ => "GUI:AINormal",
        }
    }

    /// 追加玩家名文本（可打印 ASCII，截断到上限）。
    pub fn append_name_text(&mut self, text: &str) -> bool {
        if !self.player_name_editing {
            return false;
        }
        let mut changed = false;
        for ch in text.chars() {
            if self.player_name.chars().count() >= PLAYER_NAME_MAX_CHARS {
                break;
            }
            if !is_player_name_char(ch) {
                continue;
            }
            self.player_name.push(ch);
            changed = true;
        }
        changed
    }

    /// 玩家名退格。
    pub fn backspace_name(&mut self) -> bool {
        if !self.player_name_editing || self.player_name.is_empty() {
            return false;
        }
        self.player_name.pop();
        true
    }

    /// 循环下一难度。
    pub fn cycle_difficulty(&mut self) {
        let i = LOBBY_DIFFICULTIES.iter().position(|s| *s == self.difficulty.as_str()).unwrap_or(1);
        self.difficulty = LOBBY_DIFFICULTIES[(i + 1) % LOBBY_DIFFICULTIES.len()].to_string();
    }

    /// 装载笔记片段。
    pub fn note_fragment(&self) -> String {
        format!(
            "player={} side={} diff={} map={} speed={} credits={} units={} short={}",
            self.player_name,
            self.side,
            self.difficulty,
            self.preferred_map.as_deref().unwrap_or("(auto)"),
            self.game_speed,
            self.credits,
            self.unit_count,
            self.short_game as u8
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

    /// 按下：勾选 / 滑条 / 下拉 / 玩家名。`ai_rows` 为当前地图可见 AI 行数。
    pub fn on_press(&mut self, layout: &SkirmishLobbyLayout, x: i32, y: i32, ai_rows: usize) -> Option<SkirmishLobbyHit> {
        let ai_rows = ai_rows.min(layout.ai_faces.len());
        let human_rows = (1 + ai_rows).min(layout.side_faces.len());
        // 已展开的下拉优先命中列表 / 面框。
        if self.open_combo == Some(SkirmishComboKind::Country) {
            let list = Self::country_list_rect(layout, self.combo_row);
            if list.contains(x, y) {
                let choice = ((y - list.y) / SKIRMISH_COMBO_FACE_H).clamp(0, LOBBY_SIDES.len() as i32 - 1) as usize;
                self.set_side_index(choice);
                self.open_combo = None;
                self.player_name_editing = false;
                return Some(SkirmishLobbyHit::PickCountry(choice));
            }
            if self.combo_row < layout.side_faces.len() && layout.side_faces[self.combo_row].contains(x, y) {
                self.open_combo = None;
                self.player_name_editing = false;
                return Some(SkirmishLobbyHit::ToggleCountryCombo);
            }
            self.open_combo = None;
        }
        else if self.open_combo == Some(SkirmishComboKind::Color) {
            let list = Self::color_list_rect(layout, self.combo_row);
            if list.contains(x, y) {
                let choice = ((y - list.y) / SKIRMISH_COMBO_FACE_H).clamp(0, LOBBY_COLORS.len() as i32 - 1) as usize;
                self.set_color_index(choice);
                self.open_combo = None;
                self.player_name_editing = false;
                return Some(SkirmishLobbyHit::PickColor(choice));
            }
            if self.combo_row < layout.color_faces.len() && layout.color_faces[self.combo_row].contains(x, y) {
                self.open_combo = None;
                self.player_name_editing = false;
                return Some(SkirmishLobbyHit::ToggleColorCombo);
            }
            self.open_combo = None;
        }
        else if self.open_combo == Some(SkirmishComboKind::Ai) {
            if ai_rows == 0 {
                self.open_combo = None;
            }
            else {
                let list = Self::ai_list_rect(layout);
                if list.contains(x, y) {
                    let row = ((y - list.y) / SKIRMISH_COMBO_FACE_H).clamp(0, LOBBY_DIFFICULTIES.len() as i32 - 1) as usize;
                    self.set_difficulty_index(row);
                    self.open_combo = None;
                    self.player_name_editing = false;
                    return Some(SkirmishLobbyHit::PickAi(row));
                }
                if layout.ai_faces[0].contains(x, y) {
                    self.open_combo = None;
                    self.player_name_editing = false;
                    return Some(SkirmishLobbyHit::ToggleAiCombo);
                }
                self.open_combo = None;
            }
        }

        if layout.player_name.contains(x, y) {
            self.player_name_editing = true;
            self.open_combo = None;
            return Some(SkirmishLobbyHit::FocusName);
        }
        // 点到其它左栏控件时退出编辑。
        self.player_name_editing = false;

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
        // 各行国家 / 颜色面：展开该行自己的列表。
        for row in 0..human_rows {
            if layout.side_faces[row].contains(x, y) {
                self.combo_row = row;
                self.open_combo = Some(SkirmishComboKind::Country);
                return Some(SkirmishLobbyHit::ToggleCountryCombo);
            }
            if layout.color_faces[row].contains(x, y) {
                self.combo_row = row;
                self.open_combo = Some(SkirmishComboKind::Color);
                return Some(SkirmishLobbyHit::ToggleColorCombo);
            }
        }
        // 仅当地图有 AI 席位时展开难度下拉（行 0 代表共用难度）。
        if ai_rows > 0 && layout.ai_faces[0].contains(x, y) {
            self.open_combo = Some(SkirmishComboKind::Ai);
            return Some(SkirmishLobbyHit::ToggleAiCombo);
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
    if layout.flags.iter().any(|r| r.contains(x, y)) {
        return Some("flag");
    }
    if layout.side_faces.iter().any(|r| r.contains(x, y)) {
        return Some("country");
    }
    if layout.color_faces.iter().any(|r| r.contains(x, y)) {
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

fn is_player_name_char(ch: char) -> bool {
    matches!(ch, ' '..='~')
}

fn default_row_sides() -> [u8; SKIRMISH_ROW_COUNT] {
    let mut sides = [0u8; SKIRMISH_ROW_COUNT];
    for (i, slot) in sides.iter_mut().enumerate() {
        *slot = (i % LOBBY_SIDES.len()) as u8;
    }
    sides
}

fn default_row_colors() -> [u8; SKIRMISH_ROW_COUNT] {
    let mut colors = [0u8; SKIRMISH_ROW_COUNT];
    for (i, slot) in colors.iter_mut().enumerate() {
        *slot = (i % LOBBY_COLORS.len()) as u8;
    }
    colors
}
