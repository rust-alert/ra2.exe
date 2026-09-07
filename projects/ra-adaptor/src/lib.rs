//! 按安装布局识别并装配资源表；适配能力可组合（见 `compose`）。
//!
//! 冻结定义契约在 [`ra_types::RuntimeDefinitions`]：本 crate 填充，引擎只消费。
#![deny(missing_docs)]

mod adaptor_api;
mod compose;
mod definitions;
mod layers;
mod rules;

use std::path::{Path, PathBuf};

use ra_types::{GameEdition, RaError, RaResult};

pub use adaptor_api::{Adaptor, AdaptorError, DefinitionRequest, DetectionReport};
pub use compose::{AdaptorStack, BaseGame, CapabilityReport, ExtensionId};
pub use definitions::build_runtime_definitions;
pub use layers::{
    DetectedExpansion, ExpansionFamily, MountSpec, PRIORITY_BASE_GAME, PRIORITY_EXPANSION_BASE,
    PRIORITY_MOD, PRIORITY_NESTED, PRIORITY_USER_OVERRIDE, ResourceComposition, ResourceDiagnostics,
    ResourceFile, ResourceLayer, ResourceLayerKind, compose_resource_layers, discover_expansions,
    is_expansion_mix_name, missing_base_mixes, parse_expansion_file_name,
};
pub use rules::{RulesDb, load_rules, load_rules_chain};

/// 统一资源表视图（由各 edition adaptor 填入）。
#[derive(Debug, Clone)]
pub struct ResourceChain {
    /// 当前资源链对应的 `GameEdition`。
    pub edition: GameEdition,
    /// 根目录下应存在的 MIX 文件名列表。
    pub root_mix_files: &'static [&'static str],
    /// 嵌套在根 MIX 内的子 MIX 文件名列表。
    pub nested_mix_files: &'static [&'static str],
    /// 规则 INI 在资源链中的逻辑路径。
    pub rules_ini: &'static str,
    /// 美术 INI 在资源链中的逻辑路径。
    pub art_ini: &'static str,
    /// 界面 INI 在资源链中的逻辑路径。
    pub ui_ini: &'static str,
    /// 音效 INI 在资源链中的逻辑路径。
    pub sound_ini: &'static str,
    /// 可执行文件名（用于布局校验）。
    pub exe_name: &'static str,
}

impl ResourceChain {
    /// 按 `GameEdition` 返回默认资源表（委托各 edition adaptor 的 profile）。
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
    /// 游戏安装根目录。
    pub root: PathBuf,
    /// 识别出的资源链（INI 逻辑名与嵌套表；根 MIX 以 `composition` 为准）。
    pub chain: ResourceChain,
    /// 已决议的有序资源组合（扩展发现 + 优先级挂载计划）。
    pub composition: ResourceComposition,
    /// 根目录中应按计划挂载的 MIX 文件名（`composition` 的投影，兼容旧调用方）。
    pub present_mixes: Vec<String>,
    /// 基座表中缺失的非扩展 MIX 文件名。
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
    }
    else if ra_adaptor_phobos::looks_like_mo_layout(root) {
        GameEdition::Mo3
    }
    else {
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
    let composition = compose_resource_layers(root, &chain);
    let present_mixes = composition.present_mix_names();
    let missing_mixes = missing_base_mixes(root, &chain);
    let stack = AdaptorStack::from_edition(edition).scan_extensions(root);

    Ok(EditionManifest {
        root: root.to_path_buf(),
        chain,
        composition,
        present_mixes,
        missing_mixes,
        stack,
    })
}

/// 在目录中按大小写不敏感查找文件，返回实际磁盘名。
pub fn find_ci_file(root: &Path, wanted: &str) -> Option<PathBuf> {
    let direct = root.join(wanted);
    if direct.is_file() {
        return Some(direct);
    }
    let Ok(entries) = std::fs::read_dir(root)
    else {
        return None;
    };
    let target = wanted.to_ascii_lowercase();
    for entry in entries.flatten() {
        let name = entry.file_name();
        let Some(s) = name.to_str()
        else {
            continue;
        };
        if s.to_ascii_lowercase() == target && entry.path().is_file() {
            return Some(entry.path());
        }
    }
    None
}
