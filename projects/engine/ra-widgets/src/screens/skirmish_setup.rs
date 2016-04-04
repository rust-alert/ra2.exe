//! 遭遇战大厅配置：对话框 `0x102` 选项 + 装载请求。
//!
//! 控件几何一律来自 `solve_skirmish_lobby` snapshot；本模块只持状态与命中。

use ra_layout::{
    popup_list_below, popup_list_below_min_w, RectPx, SKIRMISH_AI_ROW_COUNT, SKIRMISH_CHECK_H,
    SKIRMISH_CHECK_W, SKIRMISH_COMBO_ARROW_RESERVE, SKIRMISH_COMBO_FACE_H, SKIRMISH_ROW_COUNT,
    SKIRMISH_TRACK_ACTIVE_PAD, SKIRMISH_TRACK_PLAQUE_W,
};
use ra_layout::{solve_skirmish_lobby, LayoutSnapshot};

use crate::core::LoadKind;

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

/// 壳层阵营 chrome：以 `Sidebar.MixFileIndex` 为开放主键，不限制阵营数量。
///
/// - 侧栏包：`sidec{index:02}md.mix` / `sidec{index:02}.mix`（任意 index≥1）。
/// - 雷达名：由 `Sidebar.YuriFileNames` 决定 `radary*` 或 `radar*`。
/// - 结算图 / 调色板：优先 rules 显式键，否则共用发现池（**不按盟军/苏军/尤里猜主选**）。
///
/// 禁止再按国名或 `GDI`/`Nod`/`ThirdSide` 字符串做苏盟二元分类；一律走
/// [`UiFactionChrome::from_side_chrome`]（无表则 `None`，不静默回退 sidec01）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UiFactionChrome {
    /// 1-based，对应 `sidecNN` / `sidencNN`。
    pub mix_file_index: u32,
    /// `Sidebar.YuriFileNames`。
    pub yuri_file_names: bool,
    /// `MultiplayerScore.Background`（可空）。
    pub score_background: Option<String>,
    /// `MultiplayerScore.Palette`（可空）。
    pub score_palette: Option<String>,
    /// `EVA.Tag`（可空）。
    pub eva_tag: Option<String>,
    /// 结算统计区是否叠半透明黑底（adaptor / rules；缺省 `true`）。
    pub score_stats_shade: bool,
}

impl UiFactionChrome {
    /// 由 mix 索引构造（无结算 / EVA 覆盖）。
    pub fn from_mix_index(mix_file_index: u32, yuri_file_names: bool) -> Self {
        Self {
            mix_file_index: mix_file_index.max(1),
            yuri_file_names,
            score_background: None,
            score_palette: None,
            eva_tag: None,
            score_stats_shade: true,
        }
    }

    /// 由 Side 段键构造（开放入口：任意势力 id 只要带 `MixFileIndex` 即可）。
    pub fn from_side_keys(
        mix_file_index: Option<u32>,
        yuri_file_names: bool,
        score_background: Option<String>,
        score_palette: Option<String>,
        eva_tag: Option<String>,
        score_stats_shade: Option<bool>,
    ) -> Option<Self> {
        let index = mix_file_index.filter(|n| *n >= 1)?;
        Some(Self {
            mix_file_index: index,
            yuri_file_names,
            score_background,
            score_palette,
            eva_tag,
            score_stats_shade: score_stats_shade.unwrap_or(true),
        })
    }

    /// 由 [`ra_assets::SideChromeDef`] 构造；无可用 `MixFileIndex` 时返回 `None`。
    pub fn from_side_chrome(def: &ra_assets::SideChromeDef) -> Option<Self> {
        Self::from_side_keys(
            def.mix_file_index,
            def.yuri_file_names,
            def.score_background.clone(),
            def.score_palette.clone(),
            def.eva_tag.clone(),
            def.score_stats_shade,
        )
    }

    /// 附上结算资源覆盖。
    pub fn with_score(mut self, background: Option<String>, palette: Option<String>) -> Self {
        if background.is_some() {
            self.score_background = background;
        }
        if palette.is_some() {
            self.score_palette = palette;
        }
        self
    }

    /// 仅克隆已解析的 Side chrome；无表时返回 `None`（不静默回退 sidec01）。
    pub fn resolve(side_chrome: Option<&UiFactionChrome>) -> Option<Self> {
        side_chrome.cloned()
    }

    /// 对局侧栏基座嵌套包名。
    pub fn sidebar_mix(&self) -> String {
        format!("sidec{:02}.mix", self.mix_file_index)
    }

    /// 对局侧栏嵌套包候选（MD 优先，再基座）。
    pub fn sidebar_mix_candidates(&self) -> Vec<String> {
        let i = self.mix_file_index;
        vec![format!("sidec{:02}md.mix", i), format!("sidec{:02}.mix", i)]
    }

    /// 侧栏 / 暂停菜单雷达 SHP。
    pub fn radar_shp(&self) -> &'static str {
        if self.yuri_file_names {
            "radary.shp"
        } else {
            "radar.shp"
        }
    }

    /// 雷达调色板。
    pub fn radar_pal(&self) -> &'static str {
        if self.yuri_file_names {
            "radaryuri.pal"
        } else {
            "sidebar.pal"
        }
    }

    /// 结算战报图首选名。
    pub fn score_background_shp(&self) -> String {
        self.score_background_candidates()
            .into_iter()
            .next()
            .unwrap_or_default()
    }

    /// 结算战报调色板首选名。
    pub fn score_palette(&self) -> String {
        self.score_palette_candidates()
            .into_iter()
            .next()
            .unwrap_or_default()
    }

    /// 结算战报图候选：仅显式 `MultiplayerScore.Background`（缺则空，由 adaptor 填）。
    pub fn score_background_candidates(&self) -> Vec<String> {
        let mut out = Vec::new();
        if let Some(ref s) = self.score_background {
            push_unique_ci(&mut out, s.clone());
        }
        out
    }

    /// 结算调色板候选：仅显式 `MultiplayerScore.Palette`（缺则空，由 adaptor 填）。
    pub fn score_palette_candidates(&self) -> Vec<String> {
        let mut out = Vec::new();
        if let Some(ref s) = self.score_palette {
            push_unique_ci(&mut out, s.clone());
        }
        out
    }

    /// EVA 采样键候选：`EVA.Tag` 优先，再试库存 `Allied` / `Russian` 键（不按阵营猜优先序）。
    pub fn eva_sample_keys(&self) -> Vec<String> {
        let mut out = Vec::new();
        if let Some(tag) = self.eva_tag.as_deref().map(str::trim).filter(|s| !s.is_empty()) {
            push_unique_ci(&mut out, tag.to_string());
        }
        out
    }
}

fn push_unique_ci(out: &mut Vec<String>, name: String) {
    if out.iter().any(|s| s.eq_ignore_ascii_case(&name)) {
        return;
    }
    out.push(name);
}

/// 阵营 → 安装内旗标 PCX 候选（`local.mix` / 扩展包；前者为同优先级首选）。
///
/// 原版盘：`usai/frai/geri/gbri/japi/rusi` + 苏军三国 `djbi/arbi/lati`；
/// `cubi/lybi/iraqi` 等常见拼写不存在，仅作回退。
///
/// 模组常把改过的旗塞进 expand 里的 `lati.pcx` 等文件名，同时 CSF 把 `Confederation`
/// 显示成别国；装载时须在候选间按 MIX 优先级取胜出，不能只认基包里的首选名。
/// 在候选旗标中按「可读且 MIX/松散层优先级最高」选取；同优先级保留候选表更靠前的项。
///
/// 候选必须由调用方提供（rules `File.Flag` 或 edition adaptor 填空），本函数不猜国名。
pub fn pick_side_flag_pcx<'a>(
    candidates: &[&'a str],
    mut resolve_priority: impl FnMut(&str) -> Option<i32>,
) -> Option<&'a str> {
    let mut best: Option<(i32, usize, &'a str)> = None;
    for (index, name) in candidates.iter().copied().enumerate() {
        let name = name.trim();
        if name.is_empty() {
            continue;
        }
        let Some(priority) = resolve_priority(name) else {
            continue;
        };
        let rank = (priority, usize::MAX - index);
        let better = match best {
            None => true,
            Some((bp, bi, _)) => rank > (bp, bi),
        };
        if better {
            best = Some((priority, usize::MAX - index, name));
        }
    }
    best.map(|(_, _, name)| name)
}

/// 装载图回退调色板：共享 `mpls.pal`（非国家猜测）。
pub const LOAD_SCREEN_FALLBACK_PAL: &str = "mpls.pal";

/// 进度条 SHP（帧 0；按进度横向裁剪填充）。
pub const LOAD_SCREEN_PROGRESS_SHP: &str = "progbarm.shp";

/// 装载背景：仅使用显式文件名；空则无背景可画（不猜国名后缀）。
///
/// 若显式名为 `ls800…` 且视口较窄，尝试把前缀换成 `ls640`（同文件族缩放，非国名启发式）。
pub fn load_screen_background_shp_resolved(viewport_w: u32, rules_shp: Option<&str>) -> Option<String> {
    let name = rules_shp.map(str::trim).filter(|s| !s.is_empty())?;
    if viewport_w < 800 {
        if let Some(rest) = name.strip_prefix("ls800") {
            return Some(format!("ls640{rest}"));
        }
    }
    Some(name.to_string())
}

/// 装载调色板：仅显式名；不可读则共享 [`LOAD_SCREEN_FALLBACK_PAL`]；皆无则 `None`。
pub fn load_screen_palette_resolved(
    rules_pal: Option<&str>,
    pal_readable: impl Fn(&str) -> bool,
) -> Option<String> {
    if let Some(p) = rules_pal.map(str::trim).filter(|s| !s.is_empty()) {
        if pal_readable(p) {
            return Some(p.to_string());
        }
    }
    if pal_readable(LOAD_SCREEN_FALLBACK_PAL) {
        return Some(LOAD_SCREEN_FALLBACK_PAL.to_string());
    }
    None
}

/// 装载介绍 CSF 键：显式 `LoadScreenText.Brief` 优先；否则 `LOADBRIEF:{country_id}`（不映射后缀表）。
pub fn load_screen_brief_csf_key(country_id: &str, rules_brief: Option<&str>) -> String {
    if let Some(b) = rules_brief.map(str::trim).filter(|s| !s.is_empty()) {
        if b.contains(':') {
            return b.to_string();
        }
        return format!("LOADBRIEF:{b}");
    }
    format!("LOADBRIEF:{country_id}")
}

/// 结算战报图候选（消费已解析 chrome）。
pub fn score_screen_background_candidates(chrome: &UiFactionChrome) -> Vec<String> {
    chrome.score_background_candidates()
}

/// 结算调色板候选（消费已解析 chrome）。
pub fn score_screen_palette_candidates(chrome: &UiFactionChrome) -> Vec<String> {
    chrome.score_palette_candidates()
}

/// 结算战报图首选。
pub fn score_screen_background_shp(chrome: &UiFactionChrome) -> String {
    chrome.score_background_shp()
}

/// 结算调色板首选。
pub fn score_screen_palette(chrome: &UiFactionChrome) -> String {
    chrome.score_palette()
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
    /// 装载契约：遭遇战剥机动种 MCV；战役保留预放部队。
    pub boot_kind: LoadKind,
    /// 本地玩家名（零售默认 `Player`）。
    pub player_name: String,
    /// 优选地图文件名。
    pub preferred_map: Option<String>,
    /// 期望本地阵营（规则/地图 house 名；与 `row_sides[0]` 同步）。
    pub side: String,
    /// 可选国家短名（来自 rules `[Countries]` 的遭遇战可见子集；空表表示尚未注入）。
    pub sides: Vec<String>,
    /// 各行国家下标（相对 `sides`；行 0 本地，其后 AI）。
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
    /// 科技上限（类型 `TechLevel` 不得超过此值；默认 10）。
    pub tech_level: i32,
    /// 起始部队数。
    pub unit_count: i32,
    /// 对局随机种子（默认 0；测试夹具可显式写入，产品大厅不钉死）。
    pub match_seed: u64,
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
    /// 默认：玩家名 `Player`、无指定图（装载时按候选自动选）、普通难度；勾选对齐零售默认。
    ///
    /// 国家表由壳层从 rules 注入（[`Self::set_lobby_sides`]）。地图与席位由遭遇战大厅 / 选图页决定。
    pub fn default_lobby() -> Self {
        Self {
            boot_kind: LoadKind::Skirmish,
            player_name: "Player".to_string(),
            preferred_map: None,
            side: String::new(),
            sides: Vec::new(),
            row_sides: [0; SKIRMISH_ROW_COUNT],
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
            tech_level: 10,
            unit_count: 10,
            match_seed: 0,
            dragging: None,
            player_name_editing: false,
            open_combo: None,
            combo_row: 0,
        }
    }

    /// 写入可选国家表并钳位各行下标；保留已选国家名（若仍在新表中）。
    pub fn set_lobby_sides(&mut self, sides: Vec<String>) {
        if sides.is_empty() {
            self.sides.clear();
            self.side.clear();
            self.row_sides = [0; SKIRMISH_ROW_COUNT];
            return;
        }
        let prev: Vec<String> = (0..SKIRMISH_ROW_COUNT).map(|r| self.row_side(r).to_string()).collect();
        self.sides = sides;
        for row in 0..SKIRMISH_ROW_COUNT {
            let keep = prev.get(row).and_then(|name| {
                self.sides.iter().position(|s| s.eq_ignore_ascii_case(name))
            });
            let index = keep.unwrap_or(row % self.sides.len());
            self.row_sides[row] = index as u8;
        }
        self.side = self.row_side(0).to_string();
    }

    /// 装载时需登记的 house 列表：本地 + 当前地图席位内的 AI 行（去重保序）。
    pub fn houses_to_ensure(&self, ai_rows: usize) -> Vec<String> {
        let rows = (1 + ai_rows).min(SKIRMISH_ROW_COUNT);
        let mut out = Vec::with_capacity(rows);
        for row in 0..rows {
            let house = self.row_side(row);
            if house.is_empty() {
                continue;
            }
            if !out.iter().any(|h: &String| h.eq_ignore_ascii_case(house)) {
                out.push(house.to_string());
            }
        }
        out
    }

    /// 循环下一阵营（仅本地行）。
    pub fn cycle_side(&mut self) {
        if self.sides.is_empty() {
            return;
        }
        let i = self.row_side_index(0);
        self.set_row_side(0, (i + 1) % self.sides.len());
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
    pub fn row_side(&self, row: usize) -> &str {
        let Some(name) = self.sides.get(self.row_side_index(row))
        else {
            return "";
        };
        name.as_str()
    }

    /// 指定行色块 RGB。
    pub fn row_color_rgb(&self, row: usize) -> [u8; 3] {
        LOBBY_COLORS[self.row_color_index(row)]
    }

    fn row_side_index(&self, row: usize) -> usize {
        if self.sides.is_empty() {
            return 0;
        }
        (self.row_sides[row.min(SKIRMISH_ROW_COUNT - 1)] as usize) % self.sides.len()
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
    pub fn country_list_rect(row: usize, side_count: usize) -> RectPx {
        country_list_rect_in(&solve_skirmish_lobby(), row, side_count)
    }

    /// 颜色下拉列表矩形（紧贴指定行颜色面下方）。
    pub fn color_list_rect(row: usize) -> RectPx {
        color_list_rect_in(&solve_skirmish_lobby(), row)
    }

    /// AI 难度下拉列表矩形（紧贴行 0 AI 面下方）。
    pub fn ai_list_rect() -> RectPx {
        ai_list_rect_in(&solve_skirmish_lobby())
    }

    /// 设置指定行阵营为 `sides[index]`。
    pub fn set_row_side(&mut self, row: usize, index: usize) {
        if row >= SKIRMISH_ROW_COUNT {
            return;
        }
        if let Some(side) = self.sides.get(index) {
            self.row_sides[row] = index as u8;
            if row == 0 {
                self.side = side.clone();
            }
        }
    }

    /// 按国家短名设置指定行（找不到则忽略）。
    pub fn set_row_side_by_name(&mut self, row: usize, name: &str) {
        if let Some(index) = self.sides.iter().position(|s| s.eq_ignore_ascii_case(name)) {
            self.set_row_side(row, index);
        }
        else if row == 0 {
            self.side = name.to_string();
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
            "player={} side={} ai={} diff={} map={} seed={:#x} speed={} credits={} units={} short={}",
            self.player_name,
            self.side,
            self.row_side(1),
            self.difficulty,
            self.preferred_map.as_deref().unwrap_or("(auto)"),
            self.match_seed,
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
    pub fn on_press(&mut self, x: i32, y: i32, ai_rows: usize) -> Option<SkirmishLobbyHit> {
        let snap = solve_skirmish_lobby();
        let ai_rows = ai_rows.min(SKIRMISH_AI_ROW_COUNT);
        let human_rows = (1 + ai_rows).min(SKIRMISH_ROW_COUNT);
        // 已展开的下拉优先命中列表 / 面框。
        if self.open_combo == Some(SkirmishComboKind::Country) {
            let list = country_list_rect_in(&snap, self.combo_row, self.sides.len());
            if list.contains(x, y) && !self.sides.is_empty() {
                let choice = ((y - list.y) / SKIRMISH_COMBO_FACE_H).clamp(0, self.sides.len() as i32 - 1) as usize;
                self.set_side_index(choice);
                self.open_combo = None;
                self.player_name_editing = false;
                return Some(SkirmishLobbyHit::PickCountry(choice));
            }
            if self.combo_row < SKIRMISH_ROW_COUNT
                && snap_contains(&snap, &format!("side_face_{}", self.combo_row), x, y)
            {
                self.open_combo = None;
                self.player_name_editing = false;
                return Some(SkirmishLobbyHit::ToggleCountryCombo);
            }
            self.open_combo = None;
        }
        else if self.open_combo == Some(SkirmishComboKind::Color) {
            let list = color_list_rect_in(&snap, self.combo_row);
            if list.contains(x, y) {
                let choice = ((y - list.y) / SKIRMISH_COMBO_FACE_H).clamp(0, LOBBY_COLORS.len() as i32 - 1) as usize;
                self.set_color_index(choice);
                self.open_combo = None;
                self.player_name_editing = false;
                return Some(SkirmishLobbyHit::PickColor(choice));
            }
            if self.combo_row < SKIRMISH_ROW_COUNT
                && snap_contains(&snap, &format!("color_face_{}", self.combo_row), x, y)
            {
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
                let list = ai_list_rect_in(&snap);
                if list.contains(x, y) {
                    let row = ((y - list.y) / SKIRMISH_COMBO_FACE_H).clamp(0, LOBBY_DIFFICULTIES.len() as i32 - 1) as usize;
                    self.set_difficulty_index(row);
                    self.open_combo = None;
                    self.player_name_editing = false;
                    return Some(SkirmishLobbyHit::PickAi(row));
                }
                if snap_contains(&snap, "ai_face_0", x, y) {
                    self.open_combo = None;
                    self.player_name_editing = false;
                    return Some(SkirmishLobbyHit::ToggleAiCombo);
                }
                self.open_combo = None;
            }
        }

        if snap_contains(&snap, "player_name", x, y) {
            self.player_name_editing = true;
            self.open_combo = None;
            return Some(SkirmishLobbyHit::FocusName);
        }
        // 点到其它左栏控件时退出编辑。
        self.player_name_editing = false;

        const CHECKBOX_IDS: &[&str] = &[
            "checkbox_quick",
            "checkbox_1",
            "checkbox_2",
            "checkbox_3",
            "checkbox_4",
        ];
        for (i, id) in SkirmishCheckbox::ALL.iter().enumerate() {
            let Some(rect) = snap_rect_px(&snap, CHECKBOX_IDS[i]) else {
                continue;
            };
            let icon = RectPx::new(
                rect.x,
                rect.y,
                SKIRMISH_CHECK_W,
                SKIRMISH_CHECK_H.min(rect.h.max(SKIRMISH_CHECK_H)),
            );
            // 图标或整行标签区均可点（对齐零售勾选行为）。
            if icon.contains(x, y) || rect.contains(x, y) {
                self.set_checkbox(*id, !self.checkbox_value(*id));
                return Some(SkirmishLobbyHit::Toggle(*id));
            }
        }
        for id in [SkirmishTrackbar::GameSpeed, SkirmishTrackbar::Credits, SkirmishTrackbar::UnitCount] {
            let Some(rect) = track_rect_from_snap(&snap, id) else {
                continue;
            };
            if rect.contains(x, y) {
                self.dragging = Some(id);
                self.set_track_pos(id, track_pos_from_mouse(rect, x, id));
                return Some(SkirmishLobbyHit::Track(id));
            }
        }
        // 各行国家 / 颜色面：仅右侧箭头区展开（对齐原版 owner-draw）。
        for row in 0..human_rows {
            if let Some(face) = snap_rect_px(&snap, &format!("side_face_{row}")) {
                if combo_arrow_hit(face).contains(x, y) {
                    let same = self.open_combo == Some(SkirmishComboKind::Country) && self.combo_row == row;
                    self.combo_row = row;
                    self.open_combo = if same { None } else { Some(SkirmishComboKind::Country) };
                    return Some(SkirmishLobbyHit::ToggleCountryCombo);
                }
            }
            if let Some(face) = snap_rect_px(&snap, &format!("color_face_{row}")) {
                if combo_arrow_hit(face).contains(x, y) {
                    let same = self.open_combo == Some(SkirmishComboKind::Color) && self.combo_row == row;
                    self.combo_row = row;
                    self.open_combo = if same { None } else { Some(SkirmishComboKind::Color) };
                    return Some(SkirmishLobbyHit::ToggleColorCombo);
                }
            }
        }
        // 仅当地图有 AI 席位时展开难度下拉（行 0 代表共用难度）。
        if ai_rows > 0 {
            if let Some(face) = snap_rect_px(&snap, "ai_face_0") {
                if combo_arrow_hit(face).contains(x, y) {
                    self.open_combo = if self.open_combo == Some(SkirmishComboKind::Ai) {
                        None
                    } else {
                        Some(SkirmishComboKind::Ai)
                    };
                    return Some(SkirmishLobbyHit::ToggleAiCombo);
                }
            }
        }
        None
    }

    /// 拖动滑条。
    pub fn on_drag(&mut self, x: i32, _y: i32) -> bool {
        let Some(id) = self.dragging
        else {
            return false;
        };
        let Some(rect) = track_rect_from_snap(&solve_skirmish_lobby(), id) else {
            return false;
        };
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
pub fn hover_entry_at(x: i32, y: i32) -> Option<&'static str> {
    hover_entry_in(&solve_skirmish_lobby(), x, y)
}

fn hover_entry_in(snap: &LayoutSnapshot, x: i32, y: i32) -> Option<&'static str> {
    if snap_contains(snap, "player_name", x, y) {
        return Some("player_name");
    }
    for i in 0..SKIRMISH_ROW_COUNT {
        if snap_contains(snap, &format!("flag_{i}"), x, y) {
            return Some("flag");
        }
    }
    for i in 0..SKIRMISH_ROW_COUNT {
        if snap_contains(snap, &format!("side_face_{i}"), x, y) {
            return Some("country");
        }
    }
    for i in 0..SKIRMISH_ROW_COUNT {
        if snap_contains(snap, &format!("color_face_{i}"), x, y) {
            return Some("color");
        }
    }
    for i in 0..SKIRMISH_AI_ROW_COUNT {
        if snap_contains(snap, &format!("ai_face_{i}"), x, y) {
            return Some("ai");
        }
    }
    const CHECKBOXES: &[(&str, &str)] = &[
        ("checkbox_quick", "short_game"),
        ("checkbox_1", "mcv_repacks"),
        ("checkbox_2", "crates"),
        ("checkbox_3", "superweapons"),
        ("checkbox_4", "build_off_ally"),
    ];
    for (snap_id, entry) in CHECKBOXES {
        if let Some(rect) = snap_rect_px(snap, snap_id) {
            let icon = RectPx::new(
                rect.x,
                rect.y,
                SKIRMISH_CHECK_W,
                SKIRMISH_CHECK_H.min(rect.h.max(SKIRMISH_CHECK_H)),
            );
            if icon.contains(x, y) || rect.contains(x, y) {
                return Some(*entry);
            }
        }
    }
    if snap_contains(snap, "track_speed", x, y) || snap_contains(snap, "label_speed", x, y) {
        return Some("speed");
    }
    if snap_contains(snap, "track_credits", x, y) || snap_contains(snap, "label_credits", x, y) {
        return Some("credits");
    }
    if snap_contains(snap, "track_units", x, y) || snap_contains(snap, "label_units", x, y) {
        return Some("units");
    }
    if snap_contains(snap, "map_preview", x, y) {
        return Some("map_preview");
    }
    if snap_contains(snap, "game_type", x, y) {
        return Some("game_type");
    }
    if snap_contains(snap, "map_label", x, y) {
        return Some("map_label");
    }
    None
}

fn country_list_rect_in(snap: &LayoutSnapshot, row: usize, side_count: usize) -> RectPx {
    let row = row.min(SKIRMISH_ROW_COUNT.saturating_sub(1));
    let face = snap_rect_px(snap, &format!("side_face_{row}")).unwrap_or(RectPx::new(0, 0, 0, 0));
    popup_list_below(face, SKIRMISH_COMBO_FACE_H, side_count.max(1))
}

fn color_list_rect_in(snap: &LayoutSnapshot, row: usize) -> RectPx {
    let row = row.min(SKIRMISH_ROW_COUNT.saturating_sub(1));
    let face = snap_rect_px(snap, &format!("color_face_{row}")).unwrap_or(RectPx::new(0, 0, 0, 0));
    popup_list_below_min_w(face, SKIRMISH_COMBO_FACE_H, LOBBY_COLORS.len(), 28)
}

fn ai_list_rect_in(snap: &LayoutSnapshot) -> RectPx {
    let face = snap_rect_px(snap, "ai_face_0").unwrap_or(RectPx::new(0, 0, 0, 0));
    popup_list_below(face, SKIRMISH_COMBO_FACE_H, LOBBY_DIFFICULTIES.len())
}

fn snap_rect_px(snap: &LayoutSnapshot, id: &str) -> Option<RectPx> {
    let r = snap.get(id)?.layout.rect;
    Some(RectPx::new(
        r.x.round() as i32,
        r.y.round() as i32,
        r.width.round() as i32,
        r.height.round() as i32,
    ))
}

fn snap_contains(snap: &LayoutSnapshot, id: &str, x: i32, y: i32) -> bool {
    snap_rect_px(snap, id).is_some_and(|r| r.contains(x, y))
}

fn track_rect_from_snap(snap: &LayoutSnapshot, id: SkirmishTrackbar) -> Option<RectPx> {
    let key = match id {
        SkirmishTrackbar::GameSpeed => "track_speed",
        SkirmishTrackbar::Credits => "track_credits",
        SkirmishTrackbar::UnitCount => "track_units",
    };
    snap_rect_px(snap, key)
}

fn combo_arrow_hit(face: RectPx) -> RectPx {
    let w = SKIRMISH_COMBO_ARROW_RESERVE.min(face.w.max(0));
    RectPx::new(face.x + face.w - w, face.y, w, face.h)
}

fn track_pos_from_mouse(rect: RectPx, mouse_x: i32, id: SkirmishTrackbar) -> i32 {
    let max = id.max().max(1);
    // 活跃轨宽 = client_w - 50 - 13；鼠标 x 相对左缘偏 6 后映射。
    let active_w = (rect.w - SKIRMISH_TRACK_PLAQUE_W - SKIRMISH_TRACK_ACTIVE_PAD).max(1);
    let rel = (mouse_x - rect.x - 6).clamp(0, active_w);
    (rel * max + active_w / 2) / active_w
}

fn is_player_name_char(ch: char) -> bool {
    matches!(ch, ' '..='~')
}

fn default_row_colors() -> [u8; SKIRMISH_ROW_COUNT] {
    let mut colors = [0u8; SKIRMISH_ROW_COUNT];
    for (i, slot) in colors.iter_mut().enumerate() {
        *slot = (i % LOBBY_COLORS.len()) as u8;
    }
    colors
}
