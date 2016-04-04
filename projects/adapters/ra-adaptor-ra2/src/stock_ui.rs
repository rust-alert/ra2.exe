//! RA2 库存壳层 UI 映射（rules 缺扩展键时由 adaptor 填空）。
//!
//! 内核只消费开放字段。表中的 `GDI` / `Nod` **不是** 泰伯利亚 CNC 阵营设定，
//! 而是原版 RA2 `rules.ini` `[Sides]` / `Side=` 自带的内部节名（语义上分别为盟军 / 苏军）。

#![allow(missing_docs)]

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
    /// 结算统计区是否叠半透明黑底（原版盟军/苏军战报偏亮需要；尤里皮自带底框则关）。
    pub score_stats_shade: bool,
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
            score_stats_shade: true,
        },
        // 原版键 `Nod` → 苏军侧栏 / 结算 / EVA。
        StockSideChrome {
            id: "Nod",
            mix_file_index: 2,
            yuri_file_names: false,
            score_background: Some("mpsscrnl.shp"),
            score_palette: Some("mpsscrn.pal"),
            eva_tag: Some("Russian"),
            score_stats_shade: true,
        },
    ]
}

/// RA2 遭遇战国库存装载 / 旗。
///
/// 装载调色板用共享 `mpls.pal`：各国 `ls800*.shp` 在基座 `load.mix`，
/// 与 YR `loadmd.mix` 里的 `mpls*.pal` 国家盘索引布局不同，误用会霓虹花屏。
/// 尤里装载图在 `loadmd`，其调色板由 YR adaptor 填 `mpyls.pal`。
pub fn stock_country_ui() -> &'static [StockCountryUi] {
    &[
        StockCountryUi {
            id: "Americans",
            load_screen: "ls800ustates.shp",
            load_screen_pal: "mpls.pal",
            flag: "usai.pcx",
            load_brief_suffix: "USA",
        },
        StockCountryUi {
            id: "French",
            load_screen: "ls800france.shp",
            load_screen_pal: "mpls.pal",
            flag: "frai.pcx",
            load_brief_suffix: "FRENCH",
        },
        StockCountryUi {
            id: "Germans",
            load_screen: "ls800germany.shp",
            load_screen_pal: "mpls.pal",
            flag: "geri.pcx",
            load_brief_suffix: "GERMANS",
        },
        StockCountryUi {
            id: "British",
            load_screen: "ls800ukingdom.shp",
            load_screen_pal: "mpls.pal",
            flag: "gbri.pcx",
            load_brief_suffix: "BRITISH",
        },
        StockCountryUi {
            id: "Russians",
            load_screen: "ls800russia.shp",
            load_screen_pal: "mpls.pal",
            flag: "rusi.pcx",
            load_brief_suffix: "RUSSIA",
        },
        StockCountryUi {
            id: "Alliance",
            load_screen: "ls800korea.shp",
            load_screen_pal: "mpls.pal",
            flag: "japi.pcx",
            load_brief_suffix: "KOREA",
        },
        StockCountryUi {
            id: "Korea",
            load_screen: "ls800korea.shp",
            load_screen_pal: "mpls.pal",
            flag: "japi.pcx",
            load_brief_suffix: "KOREA",
        },
        StockCountryUi {
            id: "Koreans",
            load_screen: "ls800korea.shp",
            load_screen_pal: "mpls.pal",
            flag: "japi.pcx",
            load_brief_suffix: "KOREA",
        },
        StockCountryUi {
            id: "Confederation",
            load_screen: "ls800cuba.shp",
            load_screen_pal: "mpls.pal",
            flag: "djbi.pcx",
            load_brief_suffix: "CUBA",
        },
        StockCountryUi {
            id: "Cuba",
            load_screen: "ls800cuba.shp",
            load_screen_pal: "mpls.pal",
            flag: "djbi.pcx",
            load_brief_suffix: "CUBA",
        },
        StockCountryUi {
            id: "Cubans",
            load_screen: "ls800cuba.shp",
            load_screen_pal: "mpls.pal",
            flag: "djbi.pcx",
            load_brief_suffix: "CUBA",
        },
        StockCountryUi {
            id: "Arabs",
            load_screen: "ls800iraq.shp",
            load_screen_pal: "mpls.pal",
            flag: "arbi.pcx",
            load_brief_suffix: "IRAQ",
        },
        StockCountryUi {
            id: "Iraq",
            load_screen: "ls800iraq.shp",
            load_screen_pal: "mpls.pal",
            flag: "arbi.pcx",
            load_brief_suffix: "IRAQ",
        },
        StockCountryUi {
            id: "Iraqis",
            load_screen: "ls800iraq.shp",
            load_screen_pal: "mpls.pal",
            flag: "arbi.pcx",
            load_brief_suffix: "IRAQ",
        },
        StockCountryUi {
            id: "Africans",
            load_screen: "ls800libya.shp",
            load_screen_pal: "mpls.pal",
            flag: "lati.pcx",
            load_brief_suffix: "LYBIA",
        },
        StockCountryUi {
            id: "Libya",
            load_screen: "ls800libya.shp",
            load_screen_pal: "mpls.pal",
            flag: "lati.pcx",
            load_brief_suffix: "LYBIA",
        },
        StockCountryUi {
            id: "Libyans",
            load_screen: "ls800libya.shp",
            load_screen_pal: "mpls.pal",
            flag: "lati.pcx",
            load_brief_suffix: "LYBIA",
        },
    ]
}
