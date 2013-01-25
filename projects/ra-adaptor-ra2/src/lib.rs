//! 原版红色警戒 2：磁盘旁与启动期资源表。

use ra_types::GameEdition;

/// 某一版本期望的文件清单（差异优先当数据）。
#[derive(Debug, Clone)]
pub struct ResourceProfile {
    pub edition: GameEdition,
    /// 安装根目录旁应存在的主 MIX（大小写不敏感匹配）。
    pub root_mix_files: &'static [&'static str],
    /// 常见嵌套 MIX 名（位于主 MIX 内，启动后按需挂载）。
    pub nested_mix_files: &'static [&'static str],
    pub rules_ini: &'static str,
    pub art_ini: &'static str,
    pub ui_ini: &'static str,
    pub sound_ini: &'static str,
    pub exe_name: &'static str,
}

/// 原版 RA2 资源表。
pub fn profile() -> ResourceProfile {
    ResourceProfile {
        edition: GameEdition::Ra2,
        root_mix_files: &[
            "language.mix",
            "ra2.mix",
            "multi.mix",
            "theme.mix",
            "maps01.mix",
            "maps02.mix",
        ],
        nested_mix_files: &[
            "local.mix",
            "cache.mix",
            "conquer.mix",
            "generic.mix",
            "isogen.mix",
            "cameo.mix",
            "audio.mix",
        ],
        rules_ini: "rules.ini",
        art_ini: "art.ini",
        ui_ini: "ui.ini",
        sound_ini: "sound.ini",
        exe_name: "game.exe",
    }
}

/// 目录是否呈现原版特征。
pub fn looks_like(root: &std::path::Path) -> bool {
    root.join("game.exe").is_file()
        || root.join("rules.ini").is_file()
        || root.join("ra2.mix").is_file()
        || root.join("language.mix").is_file()
}
