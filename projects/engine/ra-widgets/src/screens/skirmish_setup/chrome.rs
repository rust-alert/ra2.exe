//! 遭遇战大厅配置：对话框 `0x102` 选项 + 装载请求。
//!
//! 控件几何一律来自 `solve_skirmish_lobby` snapshot；本模块只持状态与命中。

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
    ///
    /// `false` 时合成改画尤里金属统计框，而不是裸字。
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
        self.radar_shp_pal_candidates()[0].0
    }

    /// 雷达调色板。
    pub fn radar_pal(&self) -> &'static str {
        self.radar_shp_pal_candidates()[0].1
    }

    /// 雷达 SHP / 调色板候选（按 `YuriFileNames` 优先，再试另一套文件名）。
    ///
    /// 部分模组侧栏包（如仅含 `radary*` 的 `sidec03`）与 rules 标志不一致，
    /// 解码时应在同一 `sidecNN` 内按此表依次尝试，避免雷达槽留黑。
    pub fn radar_shp_pal_candidates(&self) -> [(&'static str, &'static str); 2] {
        if self.yuri_file_names {
            [("radary.shp", "radaryuri.pal"), ("radar.shp", "sidebar.pal")]
        } else {
            [("radar.shp", "sidebar.pal"), ("radary.shp", "radaryuri.pal")]
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

    /// EVA 采样 INI 列名：仅 `EVA.Tag`（如 `Allied` / `Russian` / `Yuri` / 模组自定义）。
    pub fn eva_sample_keys(&self) -> Vec<String> {
        let mut out = Vec::new();
        if let Some(tag) = self.eva_tag.as_deref().map(str::trim).filter(|s| !s.is_empty()) {
            push_unique_ci(&mut out, tag.to_string());
        }
        out
    }
}

/// 零售 EVA 采样名前缀（`eva.ini` / `evamd.ini` 列 → `ceva` / `csof` / `cyur`）。
///
/// 未知 `EVA.Tag`（模组自定义列）返回 `None`，勿默回盟军 `ceva`。
pub fn eva_voice_stem_prefix(eva_tag: &str) -> Option<&'static str> {
    let tag = eva_tag.trim();
    if tag.eq_ignore_ascii_case("Allied") {
        Some("ceva")
    } else if tag.eq_ignore_ascii_case("Russian") {
        Some("csof")
    } else if tag.eq_ignore_ascii_case("Yuri") {
        Some("cyur")
    } else {
        None
    }
}

/// 已知对局结束 EVA 事件在零售采样名中的三位序号（如 `ceva015` → `015`）。
pub fn eva_known_event_index(event_id: &str) -> Option<&'static str> {
    if event_id.eq_ignore_ascii_case("EVA_BattleControlTerminated") {
        Some("015")
    } else if event_id.eq_ignore_ascii_case("EVA_MissionAccomplished") {
        Some("013")
    } else if event_id.eq_ignore_ascii_case("EVA_MissionFailed") {
        Some("014")
    } else {
        None
    }
}

/// 按 `EVA.Tag` 生成已知事件的 bag 回退名（大小写各一）；无标签或未知事件则空。
pub fn eva_fallback_sample_names(event_id: &str, eva_tag: Option<&str>) -> Vec<String> {
    let Some(tag) = eva_tag.map(str::trim).filter(|s| !s.is_empty())
    else {
        return Vec::new();
    };
    let Some(prefix) = eva_voice_stem_prefix(tag)
    else {
        return Vec::new();
    };
    let Some(index) = eva_known_event_index(event_id)
    else {
        return Vec::new();
    };
    let lower = format!("{prefix}{index}");
    let upper = lower.to_ascii_uppercase();
    if upper == lower {
        vec![lower]
    } else {
        vec![lower, upper]
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
