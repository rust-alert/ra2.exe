//! 原版 RA2 与尤里的复仇：一等公民版本。

use std::path::{Path, PathBuf};

use crate::error::{RaError, RaResult};

/// 加载哪一套零售规则与资源。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum GameEdition {
    /// 原版红色警戒 2（`game.exe` / `rules.ini`）。
    Ra2,
    /// 尤里的复仇（`gamemd.exe` / `rulesmd.ini`）。
    Yr,
}

impl GameEdition {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Ra2 => "ra2",
            Self::Yr => "yr",
        }
    }

    pub fn parse(s: &str) -> RaResult<Self> {
        match s.trim().to_ascii_lowercase().as_str() {
            "ra2" | "vanilla" | "original" => Ok(Self::Ra2),
            "yr" | "yuri" | "yuris" | "md" => Ok(Self::Yr),
            other => Err(RaError::UnknownEdition(other.to_string())),
        }
    }
}

/// 某一版本期望的文件清单（差异优先当数据）。
#[derive(Debug, Clone)]
pub struct ResourceChain {
    pub edition: GameEdition,
    pub mix_files: &'static [&'static str],
    pub rules_ini: &'static str,
    pub art_ini: &'static str,
    pub ui_ini: &'static str,
    pub sound_ini: &'static str,
    pub exe_name: &'static str,
}

impl ResourceChain {
    pub fn for_edition(edition: GameEdition) -> Self {
        match edition {
            GameEdition::Ra2 => Self {
                edition,
                mix_files: &[
                    "lang.mix",
                    "ra2.mix",
                    "cache.mix",
                    "local.mix",
                    "audio.mix",
                    "expand01.mix",
                    "expand02.mix",
                ],
                rules_ini: "rules.ini",
                art_ini: "art.ini",
                ui_ini: "ui.ini",
                sound_ini: "sound.ini",
                exe_name: "game.exe",
            },
            GameEdition::Yr => Self {
                edition,
                mix_files: &[
                    "langmd.mix",
                    "ra2md.mix",
                    "cachemd.mix",
                    "localmd.mix",
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
            },
        }
    }
}

/// 探测到的安装布局。
#[derive(Debug, Clone)]
pub struct EditionManifest {
    pub root: PathBuf,
    pub chain: ResourceChain,
    pub present_mixes: Vec<String>,
    pub missing_mixes: Vec<String>,
}

/// 优先用显式版本；否则按目录特征探测。
pub fn detect_edition(root: &Path, explicit: Option<GameEdition>) -> RaResult<EditionManifest> {
    if !root.is_dir() {
        return Err(RaError::Io(format!("游戏目录不存在: {}", root.display())));
    }

    let edition = if let Some(e) = explicit {
        e
    } else {
        let has_yr = root.join("gamemd.exe").is_file()
            || root.join("rulesmd.ini").is_file()
            || root.join("ra2md.mix").is_file();
        let has_ra2 = root.join("game.exe").is_file()
            || root.join("rules.ini").is_file()
            || root.join("ra2.mix").is_file();
        match (has_ra2, has_yr) {
            (true, false) => GameEdition::Ra2,
            (false, true) => GameEdition::Yr,
            (true, true) => {
                return Err(RaError::AmbiguousEdition(root.display().to_string()));
            }
            (false, false) => {
                return Err(RaError::CannotDetectEdition(root.display().to_string()));
            }
        }
    };

    let chain = ResourceChain::for_edition(edition);
    let mut present = Vec::new();
    let mut missing = Vec::new();
    for name in chain.mix_files {
        if root.join(name).is_file() {
            present.push((*name).to_string());
        } else {
            missing.push((*name).to_string());
        }
    }

    Ok(EditionManifest {
        root: root.to_path_buf(),
        chain,
        present_mixes: present,
        missing_mixes: missing,
    })
}
