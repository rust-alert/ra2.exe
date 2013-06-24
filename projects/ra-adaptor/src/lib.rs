//! 按安装布局识别并装配资源表；适配能力可组合（见 `compose`）。

mod compose;
mod rules;

use std::path::{Path, PathBuf};

use ra_types::{GameEdition, RaError, RaResult};

pub use compose::{AdaptorStack, BaseGame, CapabilityReport, ExtensionId};
pub use rules::{load_rules, load_rules_chain, RulesDb};

/// 统一资源表视图（由各 edition adaptor 填入）。
#[derive(Debug, Clone)]
pub struct ResourceChain {
    pub edition: GameEdition,
    pub root_mix_files: &'static [&'static str],
    pub nested_mix_files: &'static [&'static str],
    pub rules_ini: &'static str,
    pub art_ini: &'static str,
    pub ui_ini: &'static str,
    pub sound_ini: &'static str,
    pub exe_name: &'static str,
}

impl ResourceChain {
    pub fn for_edition(edition: GameEdition) -> Self {
        match edition {
            GameEdition::Ra2 => from_ra2(ra_adaptor_ra2::profile()),
            GameEdition::Yr => from_yr(ra_adaptor_yuri::profile()),
            // `Mo3` 快捷方式：资源表由 Phobos adaptor 内的 MO 布局提供。
            GameEdition::Mo3 => from_phobos(ra_adaptor_phobos::mo_layout_profile()),
        }
    }
}

fn from_ra2(p: ra_adaptor_ra2::ResourceProfile) -> ResourceChain {
    ResourceChain {
        edition: p.edition,
        root_mix_files: p.root_mix_files,
        nested_mix_files: p.nested_mix_files,
        rules_ini: p.rules_ini,
        art_ini: p.art_ini,
        ui_ini: p.ui_ini,
        sound_ini: p.sound_ini,
        exe_name: p.exe_name,
    }
}

fn from_yr(p: ra_adaptor_yuri::ResourceProfile) -> ResourceChain {
    ResourceChain {
        edition: p.edition,
        root_mix_files: p.root_mix_files,
        nested_mix_files: p.nested_mix_files,
        rules_ini: p.rules_ini,
        art_ini: p.art_ini,
        ui_ini: p.ui_ini,
        sound_ini: p.sound_ini,
        exe_name: p.exe_name,
    }
}

fn from_phobos(p: ra_adaptor_phobos::ResourceProfile) -> ResourceChain {
    ResourceChain {
        edition: p.edition,
        root_mix_files: p.root_mix_files,
        nested_mix_files: p.nested_mix_files,
        rules_ini: p.rules_ini,
        art_ini: p.art_ini,
        ui_ini: p.ui_ini,
        sound_ini: p.sound_ini,
        exe_name: p.exe_name,
    }
}

/// 探测到的安装布局。
#[derive(Debug, Clone)]
pub struct EditionManifest {
    pub root: PathBuf,
    pub chain: ResourceChain,
    pub present_mixes: Vec<String>,
    pub missing_mixes: Vec<String>,
    /// 可组合适配栈（含扩展探测与能力缺口报告）。
    pub stack: AdaptorStack,
}

/// 优先用显式版本；否则按目录特征探测。
///
/// 探测优先级：MO 布局（归 Phobos）→ 仅 YR / 仅原版；原版与 YR 同时命中则报歧义。
/// 仅有 Phobos DLL、无 MO 布局时仍按 RA2/YR 基座探测，扩展记入 `stack`。
pub fn detect_edition(root: &Path, explicit: Option<GameEdition>) -> RaResult<EditionManifest> {
    if !root.is_dir() {
        return Err(RaError::Io(format!("游戏目录不存在: {}", root.display())));
    }

    let edition = if let Some(e) = explicit {
        e
    } else if ra_adaptor_phobos::looks_like_mo_layout(root) {
        GameEdition::Mo3
    } else {
        let has_yr = ra_adaptor_yuri::looks_like(root);
        let has_ra2 = ra_adaptor_ra2::looks_like(root);
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
    let present_missing = scan_root_mixes(root, chain.root_mix_files);
    let stack = AdaptorStack::from_edition(edition).scan_extensions(root);

    Ok(EditionManifest {
        root: root.to_path_buf(),
        chain,
        present_mixes: present_missing.0,
        missing_mixes: present_missing.1,
        stack,
    })
}

fn scan_root_mixes(root: &Path, names: &[&str]) -> (Vec<String>, Vec<String>) {
    let mut present = Vec::new();
    let mut missing = Vec::new();
    for name in names {
        if find_ci_file(root, name).is_some() {
            present.push((*name).to_string());
        } else {
            missing.push((*name).to_string());
        }
    }
    (present, missing)
}

/// 在目录中按大小写不敏感查找文件，返回实际磁盘名。
pub fn find_ci_file(root: &Path, wanted: &str) -> Option<PathBuf> {
    let direct = root.join(wanted);
    if direct.is_file() {
        return Some(direct);
    }
    let Ok(entries) = std::fs::read_dir(root) else {
        return None;
    };
    let target = wanted.to_ascii_lowercase();
    for entry in entries.flatten() {
        let name = entry.file_name();
        let Some(s) = name.to_str() else {
            continue;
        };
        if s.to_ascii_lowercase() == target && entry.path().is_file() {
            return Some(entry.path());
        }
    }
    None
}
