//! 载具 VXL 分图层诊断（供 `ra2 diagnose-mobile-vxl`）。
//!
//! 挂载安装 VFS 后，对词干或 techno 类型解析出的 `Image=` 词干做 body/turret/barrel/shadow
//! 尺寸与原点偏移取证。不写盘、不改叠画路径。

use std::path::PathBuf;

use ra_adaptor::detect_edition;
use ra_assets::IniDocument;
use ra_map::{
    MobileVxlDiagReport, diagnose_mobile_vxl, diagnose_mobile_vxl_sweep_body, diagnose_mobile_vxl_sweep_hva,
    diagnose_mobile_vxl_sweep_turret, mobile_vxl_diag_hva_sweep_frames, resolve_techno_image_key,
};
use ra_types::{AssetSource, GameEdition, RaError, RaResult};
use ra_widgets::fs_source::GameAssetSource;

/// 诊断请求。
#[derive(Debug, Clone)]
pub struct DiagnoseMobileVxlRequest {
    /// 游戏安装根目录。
    pub ra2_dir: PathBuf,
    /// 可选版本；合集盘应显式 `ra2`。
    pub edition: Option<String>,
    /// 资源词干（如 `mtnk`）；与 `type_id` 二选一，同时给出时以 `stem` 为准。
    pub stem: Option<String>,
    /// techno 类型 id（如 `MTNK` / `AMCV`）；经 rules/art `Image=` 解析词干。
    pub type_id: Option<String>,
    /// 车身朝向字节（扫表模式下作为固定值或起点配置）。
    pub body_facing: u8,
    /// 炮塔朝向字节。
    pub turret_facing: u8,
    /// HVA 帧。
    pub hva_frame: u32,
    /// 固定炮塔、扫车身朝向 `0,32,…,224`。
    pub sweep_body: bool,
    /// 固定车身、扫炮塔朝向 `0,32,…,224`。
    pub sweep_turret: bool,
    /// 固定朝向、扫 HVA 帧（见 `hva_frame_count`）。
    pub sweep_hva: bool,
    /// `--sweep-hva` 时的帧数（`0..n`）；`None` 时用默认 3 帧。
    pub hva_frame_count: Option<u32>,
}

/// 诊断结果（可含扫表多行）。
#[derive(Debug, Clone)]
pub struct DiagnoseMobileVxlResult {
    /// 探测版本。
    pub edition: String,
    /// 实际使用的词干（小写）。
    pub stem: String,
    /// 若由 `type_id` 解析，记录原始类型 id。
    pub type_id: Option<String>,
    /// 根 MIX 挂载数。
    pub mounted_root: usize,
    /// 嵌套 MIX 挂载份数。
    pub mounted_nested: usize,
    /// 一次或多次报告。
    pub reports: Vec<MobileVxlDiagReport>,
}

/// 挂载安装目录并对载具 VXL 做分图层诊断。
pub fn diagnose_mobile_vxl_install(req: &DiagnoseMobileVxlRequest) -> RaResult<DiagnoseMobileVxlResult> {
    if req.ra2_dir.as_os_str().is_empty() {
        return Err(RaError::Msg("diagnose-mobile-vxl: --path must not be empty".into()));
    }
    let sweep_flags = u8::from(req.sweep_body) + u8::from(req.sweep_turret) + u8::from(req.sweep_hva);
    if sweep_flags > 1 {
        return Err(RaError::Msg(
            "diagnose-mobile-vxl: --sweep-body, --sweep-turret, and --sweep-hva are mutually exclusive".into(),
        ));
    }

    let explicit = match req.edition.as_deref() {
        Some(s) => Some(GameEdition::parse(s)?),
        None => None,
    };
    let manifest = detect_edition(&req.ra2_dir, explicit)?;
    let mut source = GameAssetSource::new(manifest.root.clone());
    let (mounted_root, _) = source.mount_root_plan(&manifest.composition.root_mount_plan);
    let (mounted_nested, _) = source.mount_nested_plan(&manifest.composition.nested_mount_plan);

    let type_id = req.type_id.as_ref().map(|s| s.trim().to_string()).filter(|s| !s.is_empty());
    let stem = resolve_stem(&source, req.stem.as_deref(), type_id.as_deref())?;

    let reports = if req.sweep_body {
        diagnose_mobile_vxl_sweep_body(&source, &stem, req.turret_facing, req.hva_frame)
    } else if req.sweep_turret {
        diagnose_mobile_vxl_sweep_turret(&source, &stem, req.body_facing, req.hva_frame)
    } else if req.sweep_hva {
        let n = req.hva_frame_count.unwrap_or(mobile_vxl_diag_hva_sweep_frames().len() as u32);
        diagnose_mobile_vxl_sweep_hva(&source, &stem, req.body_facing, req.turret_facing, n)
    } else {
        vec![diagnose_mobile_vxl(&source, &stem, req.body_facing, req.turret_facing, req.hva_frame)]
    };

    Ok(DiagnoseMobileVxlResult {
        edition: manifest.chain.edition.as_str().to_string(),
        stem,
        type_id,
        mounted_root,
        mounted_nested,
        reports,
    })
}

fn resolve_stem(source: &dyn AssetSource, stem: Option<&str>, type_id: Option<&str>) -> RaResult<String> {
    if let Some(s) = stem.map(str::trim).filter(|s| !s.is_empty()) {
        return Ok(s.to_ascii_lowercase());
    }
    let Some(type_id) = type_id.map(str::trim).filter(|s| !s.is_empty()) else {
        return Err(RaError::Msg("diagnose-mobile-vxl: --stem or --type is required".into()));
    };
    let rules = source.read("rules.ini").ok().and_then(|b| IniDocument::parse(&b).ok());
    let art = source.read("art.ini").ok().and_then(|b| IniDocument::parse(&b).ok());
    let image_key = resolve_techno_image_key(rules.as_ref(), art.as_ref(), type_id);
    Ok(image_key.to_ascii_lowercase())
}
