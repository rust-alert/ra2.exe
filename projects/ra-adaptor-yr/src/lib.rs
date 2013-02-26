//! 尤里的复仇：磁盘旁与启动期资源表。

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

/// YR 资源表。
pub fn profile() -> ResourceProfile {
    ResourceProfile {
        edition: GameEdition::Yr,
        root_mix_files: &[
            "langmd.mix",
            "ra2md.mix",
            "multimd.mix",
            "thememd.mix",
            "mapsmd01.mix",
            "mapsmd02.mix",
            "mapsmd03.mix",
            "expandmd01.mix",
            "expandmd02.mix",
            "expandmd03.mix",
            // 合集盘常见：原版地图包仍在旁，供多人图名复用。
            "multi.mix",
            "maps01.mix",
            "maps02.mix",
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
        rules_ini: "rulesmd.ini",
        art_ini: "artmd.ini",
        ui_ini: "uimd.ini",
        sound_ini: "soundmd.ini",
        exe_name: "gamemd.exe",
    }
}

/// 目录是否呈现 YR 特征。
pub fn looks_like(root: &std::path::Path) -> bool {
    root.join("gamemd.exe").is_file()
        || root.join("rulesmd.ini").is_file()
        || root.join("ra2md.mix").is_file()
        || root.join("langmd.mix").is_file()
}
