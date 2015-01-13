//! 有序资源层：发现基础包与 `expand*.mix`，按稳定优先级组合挂载计划。
//!
//! `ResourceChain` 提供 edition 与 INI 逻辑名；本模块产出**已排序**的层与根 MIX 挂载计划。

use std::path::{Path, PathBuf};

use ra_types::GameEdition;

use crate::{ResourceChain, find_ci_file};

/// 基础层优先级（最低）。
pub const PRIORITY_BASE_GAME: i32 = 0;
/// 历史占位：嵌套包应**继承父档**内容层优先级，勿再把所有嵌套压到本常量。
///
/// 保留供旧调用与诊断对照；新路径请用 `MixVfs::mount_nested_all_from_parents`。
pub const PRIORITY_NESTED: i32 = 10;
/// 扩展层基数；实际优先级为 `PRIORITY_EXPANSION_BASE + index`。
pub const PRIORITY_EXPANSION_BASE: i32 = 100;
/// 模组层。
pub const PRIORITY_MOD: i32 = 1_000;
/// 用户覆盖层（最高；磁盘松散文件由 `AssetSource` 另行优先）。
pub const PRIORITY_USER_OVERRIDE: i32 = 2_000;

/// 资源层种类（排序键：`BaseGame < Expansion < Mod < UserOverride`）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum ResourceLayerKind {
    /// 原版 / 资料片基座包。
    BaseGame,
    /// 磁盘旁扩展包（`expand*.mix` 等）。
    Expansion,
    /// 模组层（预留）。
    Mod,
    /// 用户覆盖（预留）。
    UserOverride,
}

/// 扩展包族（文件名族；同编号时 `Plain < Md < Mo` 保证稳定序）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum ExpansionFamily {
    /// `expand.mix` / `expandNN.mix`。
    Plain,
    /// `expandmdNN.mix`。
    Md,
    /// `expandmoNN.mix`。
    Mo,
}

impl ExpansionFamily {
    fn as_str(self) -> &'static str {
        match self {
            Self::Plain => "plain",
            Self::Md => "md",
            Self::Mo => "mo",
        }
    }

    /// 该扩展族是否应纳入给定 `GameEdition` 的挂载计划。
    ///
    /// 合集盘常同时放 `expand01`（原版）与 `expandmd01`（尤里）。若 `edition=ra2` 仍挂上
    /// `expandmd*`，会以更高优先级盖掉壳层底板（如 `pudlgbgn.shp` 磁暴步兵被尤里立绘覆盖）。
    pub fn allowed_for_edition(self, edition: GameEdition) -> bool {
        match edition {
            GameEdition::Ra2 => matches!(self, Self::Plain),
            GameEdition::Yr | GameEdition::Mo3 => matches!(self, Self::Plain | Self::Md | Self::Mo),
        }
    }
}

/// 层内单个资源文件（当前主要为根目录 MIX）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResourceFile {
    /// 逻辑文件名（挂载名）。
    pub name: String,
    /// 磁盘路径（若已解析到）。
    pub path: Option<PathBuf>,
}

/// 一条有序资源层。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResourceLayer {
    /// 稳定层标识（如 `base`、`expansion.plain.01`）。
    pub id: String,
    /// 层种类。
    pub kind: ResourceLayerKind,
    /// 查找优先级（越大越优先覆盖）。
    pub priority: i32,
    /// 本层文件。
    pub files: Vec<ResourceFile>,
}

/// 根 MIX 挂载规格（低优先级在前，便于日志；实际覆盖由 priority 决定）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MountSpec {
    /// 挂载名（通常即磁盘文件名）。
    pub name: String,
    /// VFS 优先级。
    pub priority: i32,
    /// 所属层 id。
    pub layer_id: String,
}

/// 同名嵌套包打开策略。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NestedMountStrategy {
    /// 从每个含该名的父档各打开一份，叶文件按父层 priority 覆盖。
    AllParents,
}

/// 嵌套 MIX 挂载规格（由画像/配置写入计划；壳层只执行）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NestedMountSpec {
    /// 嵌套包逻辑名（如 `neutral.mix`）。
    pub name: String,
    /// 打开策略。
    pub strategy: NestedMountStrategy,
}

/// 已识别的扩展包。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DetectedExpansion {
    /// 实际磁盘文件名。
    pub file_name: String,
    /// 扩展编号（`expand.mix` → `0`，`expand01.mix` → `1`）。
    pub index: u32,
    /// 文件名族。
    pub family: ExpansionFamily,
}

/// 资源组合诊断（区分「发现」与后续「挂载 / 解析」）。
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ResourceDiagnostics {
    /// 已发现的扩展包文件名（稳定排序后）。
    pub detected_expansions: Vec<String>,
    /// 格式异常或无法解析编号的候选（不得静默丢弃）。
    pub malformed: Vec<String>,
    /// 其它说明。
    pub notes: Vec<String>,
}

impl ResourceDiagnostics {
    /// 多行人类可读摘要（供启动日志）。
    pub fn summary_lines(&self) -> Vec<String> {
        let mut lines = Vec::new();
        if self.detected_expansions.is_empty() {
            lines.push("detected: (none)".to_string());
        }
        else {
            lines.push(format!("detected: {}", self.detected_expansions.join(", ")));
        }
        for m in &self.malformed {
            lines.push(format!("malformed: {m}"));
        }
        for n in &self.notes {
            lines.push(format!("note: {n}"));
        }
        lines
    }
}

/// 已决议的资源组合（层 + 挂载计划 + 诊断）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResourceComposition {
    /// 有序层（按 priority 升序）。
    pub layers: Vec<ResourceLayer>,
    /// 根 MIX 挂载计划（priority 升序）。
    pub root_mount_plan: Vec<MountSpec>,
    /// 嵌套 MIX 挂载计划（壳层按序执行，勿再散读 `ResourceChain.nested_mix_files`）。
    pub nested_mount_plan: Vec<NestedMountSpec>,
    /// 发现期诊断。
    pub diagnostics: ResourceDiagnostics,
}

impl ResourceComposition {
    /// 计划中将挂载的根 MIX 名（兼容旧 `present_mixes` 消费方）。
    pub fn present_mix_names(&self) -> Vec<String> {
        self.root_mount_plan.iter().map(|s| s.name.clone()).collect()
    }

    /// 挂载计划摘要行（根包 + 嵌套包）。
    pub fn mount_plan_lines(&self) -> Vec<String> {
        let mut lines: Vec<String> =
            self.root_mount_plan.iter().map(|s| format!("mounted: {} priority={} layer={}", s.name, s.priority, s.layer_id)).collect();
        for n in &self.nested_mount_plan {
            let strat = match n.strategy {
                NestedMountStrategy::AllParents => "all_parents",
            };
            lines.push(format!("nested: {} strategy={strat}", n.name));
        }
        lines
    }
}

/// 解析扩展包文件名；非扩展返回 `None`，格式异常返回 `Err`。
pub fn parse_expansion_file_name(file_name: &str) -> Result<Option<DetectedExpansion>, String> {
    let lower = file_name.to_ascii_lowercase();
    let Some(stem) = lower.strip_suffix(".mix")
    else {
        return Ok(None);
    };
    if !stem.starts_with("expand") {
        return Ok(None);
    }

    if stem == "expand" {
        return Ok(Some(DetectedExpansion { file_name: file_name.to_string(), index: 0, family: ExpansionFamily::Plain }));
    }

    let rest = &stem["expand".len()..];
    if rest.is_empty() {
        return Err(format!("扩展名缺少编号: {file_name}"));
    }

    let (family, digits) = if let Some(d) = rest.strip_prefix("md") {
        (ExpansionFamily::Md, d)
    }
    else if let Some(d) = rest.strip_prefix("mo") {
        (ExpansionFamily::Mo, d)
    }
    else {
        (ExpansionFamily::Plain, rest)
    };

    if digits.is_empty() || !digits.chars().all(|c| c.is_ascii_digit()) {
        return Err(format!("扩展名编号无法解析: {file_name}"));
    }
    let index: u32 = digits.parse().map_err(|_| format!("扩展名编号溢出或非法: {file_name}"))?;

    Ok(Some(DetectedExpansion { file_name: file_name.to_string(), index, family }))
}

/// 判断静态表中的名字是否为扩展包（应从基座层剔除，改由发现流程纳入）。
pub fn is_expansion_mix_name(name: &str) -> bool {
    matches!(parse_expansion_file_name(name), Ok(Some(_)))
}

/// 扫描安装根目录中的扩展包；目录枚举顺序不影响结果。
pub fn discover_expansions(root: &Path) -> (Vec<DetectedExpansion>, Vec<String>) {
    let mut found = Vec::new();
    let mut malformed = Vec::new();
    let Ok(entries) = std::fs::read_dir(root)
    else {
        return (found, malformed);
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if !path.is_file() {
            continue;
        }
        let Some(name) = entry.file_name().to_str().map(|s| s.to_string())
        else {
            continue;
        };
        match parse_expansion_file_name(&name) {
            Ok(Some(exp)) => found.push(exp),
            Ok(None) => {}
            Err(msg) => {
                // 仅对 expand* 前缀报畸形，避免误伤其它 mix。
                if name.to_ascii_lowercase().starts_with("expand") {
                    malformed.push(msg);
                }
            }
        }
    }
    found.sort_by(|a, b| (a.index, a.family, a.file_name.to_ascii_lowercase()).cmp(&(b.index, b.family, b.file_name.to_ascii_lowercase())));
    found.dedup_by(|a, b| a.index == b.index && a.family == b.family && a.file_name.eq_ignore_ascii_case(&b.file_name));
    (found, malformed)
}

/// 由 edition 资源表与磁盘扫描构造有序组合。
pub fn compose_resource_layers(root: &Path, chain: &ResourceChain) -> ResourceComposition {
    let mut diagnostics = ResourceDiagnostics::default();

    let base_names: Vec<&str> = chain.root_mix_files.iter().copied().filter(|n| !is_expansion_mix_name(n)).collect();

    let mut base_files = Vec::new();
    let mut present_base = Vec::new();
    for name in &base_names {
        match find_ci_file(root, name) {
            Some(path) => {
                let disk_name = path.file_name().and_then(|s| s.to_str()).unwrap_or(name).to_string();
                present_base.push(disk_name.clone());
                base_files.push(ResourceFile { name: disk_name, path: Some(path) });
            }
            None => {
                diagnostics.notes.push(format!("基座 MIX 缺失: {name}"));
            }
        }
    }

    let mut layers = Vec::new();
    // 原版画像下：若合集盘旁有 `ra2md.mix`，低优先级挂上以便嵌套打开 `loadmd.mix`
    //（国家装载 `mpls*.pal`）。不得抬过基座，也不得引入 `expandmd*`。
    if chain.edition == GameEdition::Ra2 {
        if let Some(path) = find_ci_file(root, "ra2md.mix") {
            let disk_name = path.file_name().and_then(|s| s.to_str()).unwrap_or("ra2md.mix").to_string();
            layers.push(ResourceLayer {
                id: "base.md_load_pal".to_string(),
                kind: ResourceLayerKind::BaseGame,
                priority: PRIORITY_BASE_GAME - 1,
                files: vec![ResourceFile { name: disk_name, path: Some(path) }],
            });
            diagnostics.notes.push("已补充低优先级 ra2md.mix，供装载页读取 loadmd 国家调色板".into());
        }
    }
    if !base_files.is_empty() {
        layers.push(ResourceLayer {
            id: "base".to_string(),
            kind: ResourceLayerKind::BaseGame,
            priority: PRIORITY_BASE_GAME,
            files: base_files,
        });
    }

    let (expansions, malformed) = discover_expansions(root);
    diagnostics.malformed = malformed;
    let expansions: Vec<_> = expansions.into_iter().filter(|e| e.family.allowed_for_edition(chain.edition)).collect();
    diagnostics.detected_expansions = expansions.iter().map(|e| e.file_name.clone()).collect();

    for exp in &expansions {
        let priority = PRIORITY_EXPANSION_BASE.saturating_add(exp.index as i32);
        let layer_id = format!("expansion.{}.{:02}", exp.family.as_str(), exp.index);
        let path = find_ci_file(root, &exp.file_name);
        layers.push(ResourceLayer {
            id: layer_id,
            kind: ResourceLayerKind::Expansion,
            priority,
            files: vec![ResourceFile { name: exp.file_name.clone(), path }],
        });
    }

    layers.sort_by_key(|l| (l.priority, l.id.clone()));

    let mut root_mount_plan = Vec::new();
    for layer in &layers {
        for file in &layer.files {
            if file.path.is_some() {
                root_mount_plan.push(MountSpec { name: file.name.clone(), priority: layer.priority, layer_id: layer.id.clone() });
            }
        }
    }

    if expansions.is_empty() {
        diagnostics.notes.push("未发现 expand*.mix，沿用基座资源画像".to_string());
    }

    let nested_mount_plan: Vec<NestedMountSpec> = chain
        .nested_mix_files
        .iter()
        .copied()
        .map(|name| NestedMountSpec { name: name.to_string(), strategy: NestedMountStrategy::AllParents })
        .collect();

    let _ = present_base; // 已并入 layers
    ResourceComposition { layers, root_mount_plan, nested_mount_plan, diagnostics }
}

/// 基座表中应存在但磁盘缺失的非扩展 MIX 名。
pub fn missing_base_mixes(root: &Path, chain: &ResourceChain) -> Vec<String> {
    chain
        .root_mix_files
        .iter()
        .copied()
        .filter(|n| !is_expansion_mix_name(n))
        .filter(|n| find_ci_file(root, n).is_none())
        .map(|n| n.to_string())
        .collect()
}
