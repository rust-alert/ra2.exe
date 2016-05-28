//! YR 库存壳层 UI 映射（rules 缺扩展键时由 adaptor 填空）。
//!
//! `GDI` / `Nod` / `ThirdSide` 是原版 YR `rulesmd.ini` `[Sides]` 键，
//! 分别对应盟军 / 苏军 / 尤里，**不是** 泰伯利亚 CNC 阵营设定。

#![allow(missing_docs)]

pub use ra_adaptor_ra2::stock_ui::{StockCountryUi, StockSideChrome};

/// YR 可对战势力库存 chrome（按原版 `[Sides]` 键；尤里与苏军共用 `sidec02` + yuri 文件名）。
pub fn stock_side_chromes() -> &'static [StockSideChrome] {
    &[
        // 原版键 `GDI` → 盟军：仍用黑底统计区（`mpascrnl` 中心偏亮）。
        StockSideChrome {
            id: "GDI",
            mix_file_index: 1,
            yuri_file_names: false,
            score_background: Some("mpascrnl.shp"),
            score_palette: Some("mpascrn.pal"),
            eva_tag: Some("Allied"),
            score_stats_shade: true,
        },
        // 原版键 `Nod` → 苏军。
        StockSideChrome {
            id: "Nod",
            mix_file_index: 2,
            yuri_file_names: false,
            score_background: Some("mpsscrnl.shp"),
            score_palette: Some("mpsscrn.pal"),
            eva_tag: Some("Russian"),
            score_stats_shade: true,
        },
        // 原版键 `ThirdSide` → 尤里：关黑底，合成改画金属统计框；战报图 `mpyscrnl`。
        StockSideChrome {
            id: "ThirdSide",
            mix_file_index: 2,
            yuri_file_names: true,
            score_background: Some("mpyscrnl.shp"),
            score_palette: Some("mpyscrn.pal"),
            eva_tag: Some("Yuri"),
            score_stats_shade: false,
        },
    ]
}

/// YR 遭遇战国库存装载 / 旗（含尤里）。
///
/// 尤里 `ls800yuri.shp` 与 `mpyls.pal` 同在 MD 装载包；其它国仍走 RA2 基表的共享 `mpls.pal`。
pub fn stock_country_ui() -> &'static [StockCountryUi] {
    const EXTRA: &[StockCountryUi] = &[
        StockCountryUi {
            id: "YuriCountry",
            load_screen: "ls800yuri.shp",
            load_screen_pal: "mpyls.pal",
            flag: "yrii.pcx",
            load_brief_suffix: "YuriCountry",
        },
        StockCountryUi {
            id: "Yuri",
            load_screen: "ls800yuri.shp",
            load_screen_pal: "mpyls.pal",
            flag: "yrii.pcx",
            load_brief_suffix: "YuriCountry",
        },
    ];
    // 复用 RA2 表 + 尤里：用静态拼接不方便，YR 调用方合并两表。
    EXTRA
}

/// YR 完整国家 UI：RA2 基表 + 尤里扩展（调用方顺序拼接）。
pub fn stock_country_ui_all() -> impl Iterator<Item = &'static StockCountryUi> {
    ra_adaptor_ra2::stock_ui::stock_country_ui()
        .iter()
        .chain(stock_country_ui().iter())
}
