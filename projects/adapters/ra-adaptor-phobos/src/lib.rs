//! Phobos 扩展适配：扩展语义与其承载的内容布局（含心灵终结 3）。
//!
//! 心灵终结 3 不再作为独立 adaptor 维度；其资源表与探测归本 crate。

#![deny(missing_docs)]

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
    /// 多人模式表 INI 文件名。
    pub mpmodes_ini: &'static str,
    /// 遭遇战选图表（`missionsmd.pkt`，含 `[MultiMaps]` 源序）。
    pub missions_pkt: &'static str,
    /// 战役表 INI 文件名。
    pub battle_ini: &'static str,
    /// 布局特征用的主程序名。
    pub exe_name: &'static str,
}

/// 心灵终结 3 安装布局的资源表（YR 基座文件名 + MO 扩展包）。
///
/// 配置里仍可用 `edition = "mo3"` 作为快捷方式，语义是 YR 基座 + Phobos 系内容布局。
pub fn mo_layout_profile() -> ResourceProfile {
    ResourceProfile {
        edition: GameEdition::Mo3,
        root_mix_files: &[
            "language.mix",
            "langmd.mix",
            "ra2.mix",
            "ra2md.mix",
            "multimd.mix",
            "thememd.mix",
            // `expandmo*.mix` 由 adaptor 磁盘发现纳入 Expansion 层。
            "mapsmo03.mix",
            "multimo.mix",
            "movmo03.mix",
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
            "ntrlmd.mix",
            "loadmd.mix",
            "sidec01md.mix",
            "sidec02md.mix",
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
        exe_name: "MentalOmegaClient.exe",
    }
}

/// 兼容旧名：同 [`mo_layout_profile`]。
pub fn profile() -> ResourceProfile {
    mo_layout_profile()
}

/// 是否呈现 Phobos 引擎扩展痕迹（DLL 等）。
pub fn looks_like_phobos(root: &std::path::Path) -> bool {
    root.join("Phobos.dll").is_file() || root.join("Phobos.dll.inject").is_file() || root.join("Phobos.CRT.dll").is_file()
}

/// 目录是否呈现心灵终结 3 内容布局（归 Phobos 适配承载）。
pub fn looks_like_mo_layout(root: &std::path::Path) -> bool {
    root.join("MentalOmegaClient.exe").is_file()
        || root.join("RA2MO.ini").is_file()
        || root.join("expandmo99.mix").is_file()
        || root.join("expandmo97.mix").is_file()
        || root.join("mapsmo03.mix").is_file()
}

/// 是否应启用本 adaptor（Phobos DLL 或 MO 布局）。
pub fn looks_like(root: &std::path::Path) -> bool {
    looks_like_phobos(root) || looks_like_mo_layout(root)
}
