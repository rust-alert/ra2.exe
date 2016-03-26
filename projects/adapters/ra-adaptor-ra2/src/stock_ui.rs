//! RA2 库存壳层 UI 映射（rules 缺扩展键时由 adaptor 填空）。
//!
//! 内核只消费开放字段。表中的 `GDI` / `Nod` **不是** 泰伯利亚 CNC 阵营设定，
//! 而是原版 RA2 `rules.ini` `[Sides]` / `Side=` 自带的内部节名（语义上分别为盟军 / 苏军）。

#![allow(missing_docs)]

/// 遭遇战结算页壳层表现（edition 级，与阵营战报图分列）。
///
/// 战报图（`mpascrnl` / `mpsscrnl` / `mpyscrnl`）自带金属底框；原版偏亮时需半透明遮罩保表文可读，
/// 资料片画面更暗且底框更醒目时勿再叠「黑框」，否则盖住 adaptor 指定的战报皮。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct StockScoreScreenStyle {
    /// 是否在统计区叠半透明黑底 + 描边。
    pub stats_shade: bool,
}

/// 原版 RA2 结算：统计区需要遮罩（战报图中心偏亮）。
pub fn score_screen_style() -> StockScoreScreenStyle {
    StockScoreScreenStyle { stats_shade: true }
}

/// 库存势力 chrome 一行。
#[derive(Debug, Clone, Copy)]
pub struct StockSideChrome {
    /// 原版 rules `[Sides]` 键（RA2 库存为 `GDI`=盟军、`Nod`=苏军）。
    pub id: &'static str,
    /// `Sidebar.MixFileIndex`。
    pub mix_file_index: u32,
    /// `Sidebar.YuriFileNames`。
    pub yuri_file_names: bool,
    /// `MultiplayerScore.Background`。
    pub score_background: Option<&'static str>,
    /// `MultiplayerScore.Palette`。
    pub score_palette: Option<&'static str>,
    /// `EVA.Tag`。
    pub eva_tag: Option<&'static str>,
}

/// 库存国家装载 / 旗一行。
#[derive(Debug, Clone, Copy)]
pub struct StockCountryUi {
    /// 国家 id。
    pub id: &'static str,
    /// `File.LoadScreen`。
    pub load_screen: &'static str,
    /// `File.LoadScreenPAL`。
    pub load_screen_pal: &'static str,
    /// `File.Flag`。
    pub flag: &'static str,
    /// 装载介绍 CSF 后缀（拼 `LOADBRIEF:{brief}`）；空串表示用国家 id。
    pub load_brief_suffix: &'static str,
}

/// RA2 可对战势力库存 chrome（按原版 `[Sides]` 键对齐，不是 CNC 阵营表）。
pub fn stock_side_chromes() -> &'static [StockSideChrome] {
    &[
        // 原版键 `GDI` → 盟军侧栏 / 结算 / EVA。
        StockSideChrome {
            id: "GDI",
            mix_file_index: 1,
            yuri_file_names: false,
            score_background: Some("mpascrnl.shp"),
            score_palette: Some("mpascrn.pal"),
            eva_tag: Some("Allied"),
        },
        // 原版键 `Nod` → 苏军侧栏 / 结算 / EVA。
        StockSideChrome {
            id: "Nod",
            mix_file_index: 2,
            yuri_file_names: false,
            score_background: Some("mpsscrnl.shp"),
            score_palette: Some("mpsscrn.pal"),
            eva_tag: Some("Russian"),
        },
    ]
}

/// RA2 遭遇战国库存装载 / 旗。
pub fn stock_country_ui() -> &'static [StockCountryUi] {
    &[
        StockCountryUi {
            id: "Americans",
            load_screen: "ls800ustates.shp",
            load_screen_pal: "mplsu.pal",
            flag: "usai.pcx",
            load_brief_suffix: "USA",
        },
        StockCountryUi {
            id: "French",
            load_screen: "ls800france.shp",
            load_screen_pal: "mplsf.pal",
            flag: "frai.pcx",
            load_brief_suffix: "FRENCH",
        },
        StockCountryUi {
            id: "Germans",
            load_screen: "ls800germany.shp",
            load_screen_pal: "mplsg.pal",
            flag: "geri.pcx",
            load_brief_suffix: "GERMANS",
        },
        StockCountryUi {
            id: "British",
            load_screen: "ls800ukingdom.shp",
            load_screen_pal: "mplsuk.pal",
            flag: "gbri.pcx",
            load_brief_suffix: "BRITISH",
        },
        StockCountryUi {
            id: "Russians",
            load_screen: "ls800russia.shp",
            load_screen_pal: "mplsr.pal",
            flag: "rusi.pcx",
            load_brief_suffix: "RUSSIA",
        },
        StockCountryUi {
            id: "Alliance",
            load_screen: "ls800korea.shp",
            load_screen_pal: "mplsk.pal",
            flag: "japi.pcx",
            load_brief_suffix: "KOREA",
        },
        StockCountryUi {
            id: "Korea",
            load_screen: "ls800korea.shp",
            load_screen_pal: "mplsk.pal",
            flag: "japi.pcx",
            load_brief_suffix: "KOREA",
        },
        StockCountryUi {
            id: "Koreans",
            load_screen: "ls800korea.shp",
            load_screen_pal: "mplsk.pal",
            flag: "japi.pcx",
            load_brief_suffix: "KOREA",
        },
        StockCountryUi {
            id: "Confederation",
            load_screen: "ls800cuba.shp",
            load_screen_pal: "mplsc.pal",
            flag: "djbi.pcx",
            load_brief_suffix: "CUBA",
        },
        StockCountryUi {
            id: "Cuba",
            load_screen: "ls800cuba.shp",
            load_screen_pal: "mplsc.pal",
            flag: "djbi.pcx",
            load_brief_suffix: "CUBA",
        },
        StockCountryUi {
            id: "Cubans",
            load_screen: "ls800cuba.shp",
            load_screen_pal: "mplsc.pal",
            flag: "djbi.pcx",
            load_brief_suffix: "CUBA",
        },
        StockCountryUi {
            id: "Arabs",
            load_screen: "ls800iraq.shp",
            load_screen_pal: "mplsi.pal",
            flag: "arbi.pcx",
            load_brief_suffix: "IRAQ",
        },
        StockCountryUi {
            id: "Iraq",
            load_screen: "ls800iraq.shp",
            load_screen_pal: "mplsi.pal",
            flag: "arbi.pcx",
            load_brief_suffix: "IRAQ",
        },
        StockCountryUi {
            id: "Iraqis",
            load_screen: "ls800iraq.shp",
            load_screen_pal: "mplsi.pal",
            flag: "arbi.pcx",
            load_brief_suffix: "IRAQ",
        },
        StockCountryUi {
            id: "Africans",
            load_screen: "ls800libya.shp",
            load_screen_pal: "mplsl.pal",
            flag: "lati.pcx",
            load_brief_suffix: "LYBIA",
        },
        StockCountryUi {
            id: "Libya",
            load_screen: "ls800libya.shp",
            load_screen_pal: "mplsl.pal",
            flag: "lati.pcx",
            load_brief_suffix: "LYBIA",
        },
        StockCountryUi {
            id: "Libyans",
            load_screen: "ls800libya.shp",
            load_screen_pal: "mplsl.pal",
            flag: "lati.pcx",
            load_brief_suffix: "LYBIA",
        },
    ]
}
