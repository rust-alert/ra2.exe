//! 用户提供的安装内容：逻辑名 → 字节（浏览器无真实安装目录可读权）。
//!
//! JS 把安装包 / 目录里的文件喂进来；本模块按 edition 资源表挂进 [`MixVfs`]。

use std::collections::BTreeMap;

use ra_adaptor::{
    PRIORITY_BASE_GAME, PRIORITY_EXPANSION_BASE, ResourceChain, is_expansion_mix_name, parse_expansion_file_name,
};
use ra_assets::MixVfs;
use ra_types::{GameEdition, RaError, RaResult};
use wasm_bindgen::prelude::*;

#[derive(Clone)]
struct StoredFile {
    /// 摄入时的显示名（保留用户侧大小写）。
    display_name: String,
    bytes: Vec<u8>,
}

/// 内存中的安装文件袋 + 已挂载 VFS。
pub struct InstallBag {
    files: BTreeMap<String, StoredFile>,
    vfs: MixVfs,
}

impl Default for InstallBag {
    fn default() -> Self {
        Self::new()
    }
}

impl InstallBag {
    pub fn new() -> Self {
        Self { files: BTreeMap::new(), vfs: MixVfs::new() }
    }

    pub fn clear(&mut self) {
        self.files.clear();
        self.vfs = MixVfs::new();
    }

    pub fn file_count(&self) -> usize {
        self.files.len()
    }

    /// 摄入或覆盖同名文件（大小写不敏感键）。
    pub fn ingest(&mut self, name: &str, bytes: Vec<u8>) {
        let display_name = name.rsplit(['/', '\\']).next().unwrap_or(name).to_string();
        let key = display_name.to_ascii_lowercase();
        self.files.insert(key, StoredFile { display_name, bytes });
    }

    fn has_name(&self, wanted: &str) -> bool {
        self.files.contains_key(&wanted.to_ascii_lowercase())
    }

    fn take_bytes(&self, wanted: &str) -> Option<&[u8]> {
        self.files.get(&wanted.to_ascii_lowercase()).map(|f| f.bytes.as_slice())
    }

    fn resolve_display_name(&self, wanted: &str) -> Option<String> {
        self.files.get(&wanted.to_ascii_lowercase()).map(|f| f.display_name.clone())
    }

    /// 按显式或探测到的 edition 挂载根 MIX，并展开嵌套计划。
    pub fn prepare(&mut self, explicit: Option<GameEdition>) -> RaResult<PrepareReport> {
        let edition = match explicit {
            Some(e) => e,
            None => detect_edition_from_names(self)?,
        };
        let chain = ResourceChain::for_edition(edition);
        self.vfs = MixVfs::new();

        let mut present_root = Vec::new();
        let mut missing_base = Vec::new();
        let mut mounted_root = 0u32;
        let mut skipped_root = 0u32;

        let base_names: Vec<&str> = chain.root_mix_files.iter().copied().filter(|n| !is_expansion_mix_name(n)).collect();
        for name in &base_names {
            match self.take_bytes(name) {
                Some(data) if data.len() >= 64 => {
                    let mount_name = self.resolve_display_name(name).unwrap_or_else(|| (*name).to_string());
                    match self.vfs.mount_bytes_with_meta(
                        mount_name.clone(),
                        data.to_vec(),
                        PRIORITY_BASE_GAME,
                        None,
                        Some("base".into()),
                    ) {
                        Ok(()) => {
                            mounted_root += 1;
                            present_root.push(mount_name);
                        }
                        Err(_) => skipped_root += 1,
                    }
                }
                Some(_) => skipped_root += 1,
                None => missing_base.push((*name).to_string()),
            }
        }

        let expansions = discover_expansions_from_bag(self);
        for exp in expansions.into_iter().filter(|e| e.family.allowed_for_edition(edition)) {
            let priority = PRIORITY_EXPANSION_BASE.saturating_add(exp.index as i32);
            let layer_id = format!("expansion.{}.{:02}", expansion_family_label(exp.family), exp.index);
            match self.take_bytes(&exp.file_name) {
                Some(data) if data.len() >= 64 => {
                    let mount_name = self.resolve_display_name(&exp.file_name).unwrap_or_else(|| exp.file_name.clone());
                    match self.vfs.mount_bytes_with_meta(mount_name.clone(), data.to_vec(), priority, None, Some(layer_id)) {
                        Ok(()) => {
                            mounted_root += 1;
                            present_root.push(mount_name);
                        }
                        Err(_) => skipped_root += 1,
                    }
                }
                _ => skipped_root += 1,
            }
        }

        let mut nested_mounted = 0u32;
        for name in chain.nested_mix_files {
            if let Ok(n) = self.vfs.mount_nested_all_from_parents(name) {
                nested_mounted += n as u32;
            }
        }

        Ok(PrepareReport {
            edition: edition.as_str().to_string(),
            file_count: self.files.len() as u32,
            mounted_root,
            skipped_root,
            nested_mounted,
            present_root,
            missing_base,
        })
    }

    pub fn vfs(&self) -> &MixVfs {
        &self.vfs
    }
}

fn expansion_family_label(family: ra_adaptor::ExpansionFamily) -> &'static str {
    match family {
        ra_adaptor::ExpansionFamily::Plain => "plain",
        ra_adaptor::ExpansionFamily::Md => "md",
        ra_adaptor::ExpansionFamily::Mo => "mo",
    }
}

struct NamedExpansion {
    file_name: String,
    index: u32,
    family: ra_adaptor::ExpansionFamily,
}

fn discover_expansions_from_bag(bag: &InstallBag) -> Vec<NamedExpansion> {
    let mut found = Vec::new();
    for stored in bag.files.values() {
        match parse_expansion_file_name(&stored.display_name) {
            Ok(Some(exp)) => found.push(NamedExpansion {
                file_name: exp.file_name,
                index: exp.index,
                family: exp.family,
            }),
            _ => {}
        }
    }
    found.sort_by(|a, b| {
        (a.index, a.family, a.file_name.to_ascii_lowercase()).cmp(&(b.index, b.family, b.file_name.to_ascii_lowercase()))
    });
    found.dedup_by(|a, b| a.index == b.index && a.family == b.family && a.file_name.eq_ignore_ascii_case(&b.file_name));
    found
}

fn detect_edition_from_names(bag: &InstallBag) -> RaResult<GameEdition> {
    let has_mo = bag.has_name("expandmo01.mix") || bag.has_name("expandmo99.mix") || bag.has_name("momap.mix");
    if has_mo {
        return Ok(GameEdition::Mo3);
    }
    let has_yr = bag.has_name("gamemd.exe")
        || bag.has_name("rulesmd.ini")
        || bag.has_name("ra2md.mix")
        || bag.has_name("langmd.mix");
    let has_ra2 =
        bag.has_name("game.exe") || bag.has_name("rules.ini") || bag.has_name("ra2.mix") || bag.has_name("language.mix");
    match (has_ra2, has_yr) {
        (true, false) => Ok(GameEdition::Ra2),
        (false, true) => Ok(GameEdition::Yr),
        (true, true) => Err(RaError::AmbiguousEdition("imported-files".into())),
        (false, false) => Err(RaError::CannotDetectEdition("imported-files".into())),
    }
}

/// 一次 `prepare` 的摘要（供 JS 展示）。
#[derive(Debug, Clone)]
#[wasm_bindgen(js_name = PrepareReport)]
pub struct PrepareReport {
    edition: String,
    file_count: u32,
    mounted_root: u32,
    skipped_root: u32,
    nested_mounted: u32,
    present_root: Vec<String>,
    missing_base: Vec<String>,
}

#[wasm_bindgen]
impl PrepareReport {
    /// 探测或显式指定的 edition（`ra2` / `yr` / `mo3`）。
    #[wasm_bindgen(getter)]
    pub fn edition(&self) -> String {
        self.edition.clone()
    }

    /// 已摄入文件数。
    #[wasm_bindgen(getter, js_name = fileCount)]
    pub fn file_count(&self) -> u32 {
        self.file_count
    }

    /// 成功挂载的根 MIX 数量。
    #[wasm_bindgen(getter, js_name = mountedRoot)]
    pub fn mounted_root(&self) -> u32 {
        self.mounted_root
    }

    /// 存在但未能挂载的根 MIX 数量。
    #[wasm_bindgen(getter, js_name = skippedRoot)]
    pub fn skipped_root(&self) -> u32 {
        self.skipped_root
    }

    /// 从父档展开的嵌套 MIX 份数。
    #[wasm_bindgen(getter, js_name = nestedMounted)]
    pub fn nested_mounted(&self) -> u32 {
        self.nested_mounted
    }

    /// 已挂载根包显示名列表。
    #[wasm_bindgen(getter, js_name = presentRoot)]
    pub fn present_root(&self) -> Vec<String> {
        self.present_root.clone()
    }

    /// 资源表要求但袋中缺失的基座包名。
    #[wasm_bindgen(getter, js_name = missingBase)]
    pub fn missing_base(&self) -> Vec<String> {
        self.missing_base.clone()
    }

    /// 多行文本摘要（调试 / 日志）。
    pub fn summary(&self) -> String {
        format_report(self)
    }
}

/// Wasm 侧安装会话：摄入文件 → 按 edition 挂载。
#[wasm_bindgen]
pub struct InstallSession {
    bag: InstallBag,
}

#[wasm_bindgen]
impl InstallSession {
    /// 新建空会话。
    #[wasm_bindgen(constructor)]
    pub fn new() -> InstallSession {
        InstallSession { bag: InstallBag::new() }
    }

    /// 已摄入文件数。
    #[wasm_bindgen(js_name = fileCount)]
    pub fn file_count(&self) -> u32 {
        self.bag.file_count() as u32
    }

    /// 清空已摄入文件与 VFS。
    pub fn clear(&mut self) {
        self.bag.clear();
    }

    /// 摄入单个安装文件（`name` 可为路径，仅取末段文件名）。
    #[wasm_bindgen(js_name = ingestFile)]
    pub fn ingest_file(&mut self, name: &str, data: &[u8]) {
        self.bag.ingest(name, data.to_vec());
    }

    /// 按 edition 挂载；`edition` 为空则自动探测。
    #[wasm_bindgen(js_name = prepareEdition)]
    pub fn prepare_edition(&mut self, edition: Option<String>) -> Result<PrepareReport, JsValue> {
        let explicit = match edition.as_deref().map(str::trim).filter(|s| !s.is_empty()) {
            Some(s) => Some(GameEdition::parse(s).map_err(|e| JsValue::from_str(&e.to_string()))?),
            None => None,
        };
        self.bag.prepare(explicit).map_err(|e| JsValue::from_str(&e.to_string()))
    }

    /// 从已挂载 VFS 按逻辑名读取字节（未挂载或缺失则空）。
    #[wasm_bindgen(js_name = readLogical)]
    pub fn read_logical(&self, name: &str) -> Option<Vec<u8>> {
        self.bag.vfs().read(name)
    }
}

impl Default for InstallSession {
    fn default() -> Self {
        Self::new()
    }
}

fn format_report(report: &PrepareReport) -> String {
    let present = if report.present_root.is_empty() {
        "(none)".into()
    } else {
        report.present_root.join(", ")
    };
    let missing = if report.missing_base.is_empty() {
        "(none)".into()
    } else {
        report.missing_base.join(", ")
    };
    format!(
        "edition={}\nfiles={}\nmounted_root={}\nskipped_root={}\nnested_mounted={}\npresent={}\nmissing_base={}",
        report.edition, report.file_count, report.mounted_root, report.skipped_root, report.nested_mounted, present, missing
    )
}

/// 已摄入文件数（无会话时为 0；兼容 host 占位 API）。
pub fn ingested_file_count() -> usize {
    0
}

#[cfg(test)]
mod tests {
    use super::*;

    fn bag_named(names: &[&str]) -> InstallBag {
        let mut bag = InstallBag::new();
        for name in names {
            bag.ingest(name, vec![0u8; 64]);
        }
        bag
    }

    #[test]
    fn detect_ra2_from_classic_markers() {
        let bag = bag_named(&["RA2.MIX", "language.mix", "game.exe"]);
        assert_eq!(detect_edition_from_names(&bag).unwrap(), GameEdition::Ra2);
    }

    #[test]
    fn detect_yr_from_md_markers() {
        let bag = bag_named(&["ra2md.mix", "langmd.mix", "gamemd.exe"]);
        assert_eq!(detect_edition_from_names(&bag).unwrap(), GameEdition::Yr);
    }

    #[test]
    fn ambiguous_when_both_ra2_and_yr_markers() {
        let bag = bag_named(&["ra2.mix", "ra2md.mix"]);
        assert!(matches!(detect_edition_from_names(&bag), Err(RaError::AmbiguousEdition(_))));
    }

    #[test]
    fn prepare_explicit_edition_reports_missing_base() {
        let mut bag = bag_named(&["expand01.mix"]);
        let report = bag.prepare(Some(GameEdition::Ra2)).unwrap();
        assert_eq!(report.edition(), "ra2");
        assert!(report.missing_base().iter().any(|n| n.eq_ignore_ascii_case("ra2.mix")));
        assert!(report.missing_base().iter().any(|n| n.eq_ignore_ascii_case("language.mix")));
        assert!(report.summary().contains("edition=ra2"));
    }
}
