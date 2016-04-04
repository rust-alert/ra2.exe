//! 尤里的复仇：磁盘旁与启动期资源表。

#![deny(missing_docs)]

pub mod stock_ui;

use ra_types::GameEdition;

/// 与 `ra-adaptor-ra2::ResourceProfile` 同形，避免跨 crate 循环依赖。
#[derive(Debug, Clone)]
pub struct ResourceProfile {
    /// 对应的 `GameEdition`。
    pub edition: GameEdition,
    /// 安装根目录旁应存在的主 MIX。
    pub root_mix_files: &'static [&'static str],
    /// 常见嵌套 MIX 名。
    pub nested_mix_files: &'static [&'static str],
    /// rules INI 文件名。
    pub rules_ini: &'static str,
    /// art INI 文件名。
    pub art_ini: &'static str,
    /// UI INI 文件名。
    pub ui_ini: &'static str,
    /// 音效 INI 文件名。
    pub sound_ini: &'static str,
    /// 多人模式表 INI 文件名（`mpmodesmd.ini`）。
    pub mpmodes_ini: &'static str,
    /// 遭遇战选图表（`missionsmd.pkt`，含 `[MultiMaps]` 源序）。
    pub missions_pkt: &'static str,
    /// 战役表 INI 文件名（`battlemd.ini`）。
    pub battle_ini: &'static str,
    /// 布局特征用的主程序名。
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
            // `expandmd*.mix` 由 adaptor 磁盘发现纳入 Expansion 层，不写死进基座表。
            // 合集盘常见：原版语言/主包/地图包仍在旁。闪屏 `title.pcx`、部分 CSF/字体在 `language.mix` / `ra2.mix`。
            "language.mix",
            "ra2.mix",
            "theme.mix",
            "multi.mix",
            "maps01.mix",
            "maps02.mix",
        ],
        // 壳层 MD 对：`ntrlmd` / `loadmd` / `sidec*md`；基座 `neutral.mix` 等由磁盘旁 `ra2.mix` 提供时一并挂上。
        // 同优先级后挂载覆盖：基座嵌套在前、MD 在后，避免 `load.mix` 盖掉 `loadmd` 装载图。
        nested_mix_files: &[
            "local.mix",
            "cache.mix",
            "conquer.mix",
            "generic.mix",
            "isogen.mix",
            "cameo.mix",
            "audio.mix",
            "neutral.mix",
            "load.mix",
            "sidec01.mix",
            "sidec02.mix",
            "sidenc01.mix",
            "sidenc02.mix",
            "localmd.mix",
            "cachemd.mix",
            "conqmd.mix",
            "genermd.mix",
            "isogenmd.mix",
            "cameomd.mix",
            "audiomd.mix",
            "ntrlmd.mix",
            "loadmd.mix",
            "sidec01md.mix",
            "sidec02md.mix",
            "expandmd01.mix",
            "expandmd02.mix",
            "expandmd03.mix",
        ],
        rules_ini: "rulesmd.ini",
        art_ini: "artmd.ini",
        ui_ini: "uimd.ini",
        sound_ini: "soundmd.ini",
        mpmodes_ini: "mpmodesmd.ini",
        missions_pkt: "missionsmd.pkt",
        battle_ini: "battlemd.ini",
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
