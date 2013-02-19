//! 心灵终结 3（Mental Omega 3）：磁盘旁与启动期资源表。

use ra_types::GameEdition;

/// 与 `ra-adaptor-ra2::ResourceProfile` 同形，避免跨 crate 循环依赖。
#[derive(Debug, Clone)]
pub struct ResourceProfile {
    pub edition: GameEdition,
    pub root_mix_files: &'static [&'static str],
    pub nested_mix_files: &'static [&'static str],
    pub rules_ini: &'static str,
    pub art_ini: &'static str,
    pub ui_ini: &'static str,
    pub sound_ini: &'static str,
    pub exe_name: &'static str,
}

/// 心灵终结 3 资源表（YR 基座文件名 + MO 扩展包）。
pub fn profile() -> ResourceProfile {
    ResourceProfile {
        edition: GameEdition::Mo3,
        root_mix_files: &[
            // YR / RA2 基座（MO 官方安装说明要求存在）
            "language.mix",
            "langmd.mix",
            "ra2.mix",
            "ra2md.mix",
            "multimd.mix",
            "thememd.mix",
            // MO 3.3 扩展与地图包
            "expandmo95.mix",
            "expandmo96.mix",
            "expandmo97.mix",
            "expandmo99.mix",
            "mapsmo03.mix",
            "multimo.mix",
            "movmo03.mix",
            // 可选：语言包 / 原声
            "expandmo98.mix",
            "expandmo94.mix",
            "thememo.mix",
        ],
        nested_mix_files: &[
            "localmd.mix",
            "cachemd.mix",
            "conqmd.mix",
            "genermd.mix",
            "isogenmd.mix",
            "cameomd.mix",
            "audiomd.mix",
            "expandmd01.mix",
            "expandmd02.mix",
            "expandmd03.mix",
        ],
        // Ares 仍走 md 系 INI 名；覆写内容在 expandmo* 内。
        rules_ini: "rulesmd.ini",
        art_ini: "artmd.ini",
        ui_ini: "uimd.ini",
        sound_ini: "soundmd.ini",
        exe_name: "MentalOmegaClient.exe",
    }
}

/// 目录是否呈现心灵终结 3 特征。
pub fn looks_like(root: &std::path::Path) -> bool {
    root.join("MentalOmegaClient.exe").is_file()
        || root.join("RA2MO.ini").is_file()
        || root.join("expandmo99.mix").is_file()
        || root.join("expandmo97.mix").is_file()
        || root.join("mapsmo03.mix").is_file()
}
