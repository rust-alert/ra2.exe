//! edition 库存壳层 UI：填 rules 缺键，不进引擎苏盟分类。

use ra_assets::{CountryDef, SideChromeDef, fill_country_ui_gaps, fill_side_chrome_gaps};
use ra_types::GameEdition;

use crate::ResourceChain;

fn side_from_stock(s: &ra_adaptor_ra2::stock_ui::StockSideChrome) -> SideChromeDef {
    SideChromeDef {
        id: s.id.to_string(),
        mix_file_index: Some(s.mix_file_index),
        yuri_file_names: s.yuri_file_names,
        score_background: s.score_background.map(str::to_string),
        score_palette: s.score_palette.map(str::to_string),
        eva_tag: s.eva_tag.map(str::to_string),
        score_stats_shade: Some(s.score_stats_shade),
    }
}

fn country_from_stock(s: &ra_adaptor_ra2::stock_ui::StockCountryUi) -> CountryDef {
    let brief = if s.load_brief_suffix.is_empty() {
        String::new()
    } else if s.load_brief_suffix.contains(':') {
        s.load_brief_suffix.to_string()
    } else {
        format!("LOADBRIEF:{}", s.load_brief_suffix)
    };
    CountryDef {
        id: s.id.to_string(),
        list_index: 0,
        ui_name: String::new(),
        prefix: String::new(),
        color: String::new(),
        side: String::new(),
        multiplay: false,
        multiplay_obsolete: false,
        special_ui_name: String::new(),
        load_screen: s.load_screen.to_string(),
        load_screen_pal: s.load_screen_pal.to_string(),
        flag: s.flag.to_string(),
        load_brief: brief,
    }
}

/// 按 edition 用 adaptor 库存表填空 `CountryDef` / `SideChromeDef`。
///
/// MO 等 rules 已写全键的布局：传入后几乎 no-op。RA2/YR 补 MixFileIndex 与装载艺术。
pub fn apply_edition_stock_ui(edition: GameEdition, countries: &mut [CountryDef], chromes: &mut [SideChromeDef]) {
    match edition {
        GameEdition::Ra2 => {
            let sides: Vec<_> = ra_adaptor_ra2::stock_ui::stock_side_chromes()
                .iter()
                .map(side_from_stock)
                .collect();
            let ui: Vec<_> = ra_adaptor_ra2::stock_ui::stock_country_ui()
                .iter()
                .map(country_from_stock)
                .collect();
            fill_side_chrome_gaps(chromes, &sides);
            fill_country_ui_gaps(countries, &ui);
        }
        GameEdition::Yr => {
            let sides: Vec<_> = ra_adaptor_yuri::stock_ui::stock_side_chromes()
                .iter()
                .map(side_from_stock)
                .collect();
            let ui: Vec<_> = ra_adaptor_yuri::stock_ui::stock_country_ui_all()
                .map(country_from_stock)
                .collect();
            fill_side_chrome_gaps(chromes, &sides);
            fill_country_ui_gaps(countries, &ui);
        }
        GameEdition::Mo3 => {
            // rulesmo 等已带 MixFileIndex / File.LoadScreen；不注入 RA2/YR 库存表。
        }
    }
}

/// 便捷：对当前 [`ResourceChain`] 的 edition 填空。
pub fn apply_stock_ui_for_chain(chain: &ResourceChain, countries: &mut [CountryDef], chromes: &mut [SideChromeDef]) {
    apply_edition_stock_ui(chain.edition, countries, chromes);
}
