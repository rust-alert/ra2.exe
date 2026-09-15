//! N-API 入口：`version` + `emulate` + `extract` + `unpack` + `diagnoseMaps` + `diagnoseMobileVxl`。
//!
//! 原生窗口与事件循环在 [`host`]。

#![deny(clippy::all)]
#![allow(missing_docs)]

pub mod host;

use std::path::PathBuf;

use napi::bindgen_prelude::*;
use napi_derive::napi;
use ra_config::EmulateOverride;

use crate::host::{
    diagnose_maps::{DiagnoseMapsRequest, diagnose_skirmish_maps},
    diagnose_mobile_vxl::{DiagnoseMobileVxlRequest, diagnose_mobile_vxl_install},
    extract::{ExtractRequest, UnpackRequest, extract_named, unpack_all},
};

/// 绑定版本字符串。
#[napi]
pub fn version() -> String {
    env!("CARGO_PKG_VERSION").to_string()
}

/// `ra2 emulate` 选项。
#[napi(object)]
pub struct EmulateOptions {
    /// 游戏安装根目录（含 MIX / INI）。
    pub path: String,
    /// 可选版本：`ra2` / `yr` 等。
    pub edition: Option<String>,
    /// 可选启动产品页：`skirmish` / `main` / `campaign` 等（跳过闪屏）。
    pub screen: Option<String>,
}

/// 注入路径覆盖并阻塞进入 GUI 事件循环。
#[napi]
pub fn emulate(options: EmulateOptions) -> Result<()> {
    let path = PathBuf::from(options.path.trim());
    if path.as_os_str().is_empty() {
        return Err(Error::from_reason("--path must not be empty"));
    }
    ra_config::set_emulate_override(EmulateOverride {
        ra2_dir: path,
        edition: options.edition.filter(|s| !s.trim().is_empty()),
        screen: options.screen.filter(|s| !s.trim().is_empty()),
    });
    host::run().map_err(|e| Error::from_reason(format!("{e}")))
}

/// `ra2 extract` 选项。
#[napi(object)]
pub struct ExtractOptions {
    /// 游戏安装根目录。
    pub path: String,
    /// 可选版本：`ra2` / `yr` 等。
    pub edition: Option<String>,
    /// 输出目录。
    pub out: String,
    /// 逻辑文件名列表（如 `sdtp.shp`）。
    pub names: Vec<String>,
    /// 可选调色板逻辑名（解码 SHP 时优先）。
    pub palette: Option<String>,
    /// 为 `.shp` / 剧院地形 SHP 额外写出各帧 PNG。
    pub decode_shp: Option<bool>,
    /// 为 `.csf` 额外写出 UTF-8 `KEY=value` 文本表（`name.txt`）。
    pub decode_csf: Option<bool>,
    /// 可选剧院（挂载 `isotemp.mix` 等）；缺省时从请求名扩展名推断。
    pub theater: Option<String>,
}

/// 单个已导出文件（N-API）。
#[napi(object)]
pub struct ExtractedFileJs {
    /// 逻辑名。
    pub name: String,
    /// 落盘路径。
    pub path: String,
    /// 字节数。
    pub bytes: u32,
    /// 来源说明。
    pub origin: String,
    /// SHP 族解析得到的总帧数（未解析则为 `null`）。
    pub shp_frames: Option<u32>,
    /// SHP 族画布宽（未解析则为 `null`）。
    pub shp_width: Option<u32>,
    /// SHP 族画布高（未解析则为 `null`）。
    pub shp_height: Option<u32>,
    /// CSF 已解码条目数（未解码则为 `null`）。
    pub csf_entries: Option<u32>,
}

/// 按名导出结果（N-API）。
#[napi(object)]
pub struct ExtractResultJs {
    /// 成功写出。
    pub written: Vec<ExtractedFileJs>,
    /// 缺失名。
    pub missing: Vec<String>,
    /// 探测版本。
    pub edition: String,
    /// 根 MIX 挂载数。
    pub mounted_root: u32,
    /// 嵌套 MIX 挂载份数。
    pub mounted_nested: u32,
}

/// 按逻辑名从安装 VFS 导出原始字节（可选 SHP→PNG / CSF→TXT）。
#[napi]
pub fn extract(options: ExtractOptions) -> Result<ExtractResultJs> {
    let path = PathBuf::from(options.path.trim());
    let out = PathBuf::from(options.out.trim());
    if path.as_os_str().is_empty() {
        return Err(Error::from_reason("extract: --path must not be empty"));
    }
    if out.as_os_str().is_empty() {
        return Err(Error::from_reason("extract: --out must not be empty"));
    }
    if options.names.is_empty() {
        return Err(Error::from_reason("extract: at least one name is required"));
    }

    let req = ExtractRequest {
        ra2_dir: path,
        edition: options.edition.filter(|s| !s.trim().is_empty()),
        out_dir: out,
        names: options.names,
        palette: options.palette.filter(|s| !s.trim().is_empty()),
        decode_shp: options.decode_shp.unwrap_or(false),
        decode_csf: options.decode_csf.unwrap_or(false),
        theater: options.theater.filter(|s| !s.trim().is_empty()),
    };

    let report = extract_named(&req).map_err(|e| Error::from_reason(format!("{e}")))?;
    Ok(ExtractResultJs {
        written: report
            .written
            .into_iter()
            .map(|f| ExtractedFileJs {
                name: f.name,
                path: f.path.display().to_string(),
                bytes: f.bytes as u32,
                origin: f.origin,
                shp_frames: f.shp_frames.map(|n| n as u32),
                shp_width: f.shp_width.map(u32::from),
                shp_height: f.shp_height.map(u32::from),
                csf_entries: f.csf_entries.map(|n| n as u32),
            })
            .collect(),
        missing: report.missing,
        edition: report.edition,
        mounted_root: report.mounted_root as u32,
        mounted_nested: report.mounted_nested as u32,
    })
}

/// `ra2 unpack` 选项（全量解包，无文件名列表）。
#[napi(object)]
pub struct UnpackOptions {
    /// 游戏安装根目录。
    pub path: String,
    /// 可选版本：`ra2` / `yr` 等。
    pub edition: Option<String>,
    /// 输出根目录。
    pub out: String,
    /// 可选额外文件名表（一行一个逻辑名）。
    pub names_file: Option<String>,
    /// 遇到 `.csf` 时额外写出同名 `.txt` 文本表。
    pub decode_csf: Option<bool>,
}

/// 全量解包结果（N-API）。
#[napi(object)]
pub struct UnpackResultJs {
    /// 写出条目数。
    pub files_written: u32,
    /// 恢复原名后写出的条目数。
    pub named_written: u32,
    /// 仍以哈希 id 落盘的条目数。
    pub unnamed_written: u32,
    /// 累计字节。
    pub bytes_written: f64,
    /// 写出的档案子目录数。
    pub archives: u32,
    /// 恢复表登记条数。
    pub name_table_size: u32,
    /// 探测版本。
    pub edition: String,
    /// 根 MIX 挂载数。
    pub mounted_root: u32,
    /// 嵌套 MIX 挂载份数。
    pub mounted_nested: u32,
    /// 输出根目录。
    pub out_dir: String,
}

/// 全量解包已挂载 MIX 树中的全部索引条目（按档案分子目录；原名来自哈希恢复表）。
#[napi]
pub fn unpack(options: UnpackOptions) -> Result<UnpackResultJs> {
    let path = PathBuf::from(options.path.trim());
    let out = PathBuf::from(options.out.trim());
    if path.as_os_str().is_empty() {
        return Err(Error::from_reason("unpack: --path must not be empty"));
    }
    if out.as_os_str().is_empty() {
        return Err(Error::from_reason("unpack: --out must not be empty"));
    }

    let req = UnpackRequest {
        ra2_dir: path,
        edition: options.edition.filter(|s| !s.trim().is_empty()),
        out_dir: out,
        names_file: options.names_file.filter(|s| !s.trim().is_empty()).map(PathBuf::from),
        decode_csf: options.decode_csf.unwrap_or(false),
    };
    let report = unpack_all(&req).map_err(|e| Error::from_reason(format!("{e}")))?;
    Ok(UnpackResultJs {
        files_written: report.files_written as u32,
        named_written: report.named_written as u32,
        unnamed_written: report.unnamed_written as u32,
        bytes_written: report.bytes_written as f64,
        archives: report.archives as u32,
        name_table_size: report.name_table_size as u32,
        edition: report.edition,
        mounted_root: report.mounted_root as u32,
        mounted_nested: report.mounted_nested as u32,
        out_dir: report.out_dir.display().to_string(),
    })
}

/// `ra2 diagnose-maps` 选项。
#[napi(object)]
pub struct DiagnoseMapsOptions {
    /// 游戏安装根目录。
    pub path: String,
    /// 可选版本：`ra2` / `yr` 等（合集盘须显式 `ra2`）。
    pub edition: Option<String>,
    /// 最多诊断前 N 张；缺省为全表。
    pub limit: Option<u32>,
}

/// 单张地图诊断行（N-API）。
#[napi(object)]
pub struct MapDiagnoseRowJs {
    pub file_name: String,
    pub name_csf: String,
    pub parse_ok: bool,
    pub parse_error: Option<String>,
    pub prepare_ok: bool,
    pub prepare_error: Option<String>,
    pub blocking_gaps: Vec<String>,
    pub stub_gaps: Vec<String>,
    pub deferred_gaps: Vec<String>,
    pub other_gaps: Vec<String>,
    /// `success` / `reject` / `missing`（装载／准备口径，非整局可玩）。
    pub tri_state: String,
}

/// 遭遇战地图包诊断报告（N-API）。
#[napi(object)]
pub struct DiagnoseMapsReportJs {
    pub edition: String,
    pub source: String,
    pub candidate_count: u32,
    /// 规则层能力缺口（全表共用，如超武有定义无执行器）。
    pub rules_capability_gaps: Vec<String>,
    pub maps: Vec<MapDiagnoseRowJs>,
    pub success: u32,
    pub reject: u32,
    pub missing: u32,
}

/// 枚举 `missions.pkt`（或扫描回退）并对每张图做解析／准备／能力缺口三态诊断。
#[napi]
pub fn diagnose_maps(options: DiagnoseMapsOptions) -> Result<DiagnoseMapsReportJs> {
    let path = PathBuf::from(options.path.trim());
    if path.as_os_str().is_empty() {
        return Err(Error::from_reason("diagnose-maps: --path must not be empty"));
    }
    let req = DiagnoseMapsRequest {
        ra2_dir: path,
        edition: options.edition.filter(|s| !s.trim().is_empty()),
        limit: options.limit.map(|n| n as usize),
    };
    let report = diagnose_skirmish_maps(&req).map_err(|e| Error::from_reason(format!("{e}")))?;
    Ok(DiagnoseMapsReportJs {
        edition: report.edition,
        source: report.source,
        candidate_count: report.candidate_count as u32,
        rules_capability_gaps: report.rules_capability_gaps,
        maps: report
            .maps
            .into_iter()
            .map(|m| MapDiagnoseRowJs {
                file_name: m.file_name,
                name_csf: m.name_csf,
                parse_ok: m.parse_ok,
                parse_error: m.parse_error,
                prepare_ok: m.prepare_ok,
                prepare_error: m.prepare_error,
                blocking_gaps: m.blocking_gaps,
                stub_gaps: m.stub_gaps,
                deferred_gaps: m.deferred_gaps,
                other_gaps: m.other_gaps,
                tri_state: m.tri_state,
            })
            .collect(),
        success: report.success as u32,
        reject: report.reject as u32,
        missing: report.missing as u32,
    })
}

/// `ra2 diagnose-mobile-vxl` 选项。
#[napi(object)]
pub struct DiagnoseMobileVxlOptions {
    /// 游戏安装根目录。
    pub path: String,
    /// 可选版本：`ra2` / `yr` 等（合集盘须显式 `ra2`）。
    pub edition: Option<String>,
    /// 资源词干（如 `mtnk`）；与 `type_id` 至少提供一个。
    pub stem: Option<String>,
    /// techno 类型 id（如 `MTNK`）；经 `Image=` 解析词干。
    pub type_id: Option<String>,
    /// 车身朝向字节；缺省 `0`。
    pub body_facing: Option<u32>,
    /// 炮塔朝向字节；缺省与车身相同，未给时用 `body_facing`。
    pub turret_facing: Option<u32>,
    /// HVA 帧；缺省 `0`。
    pub hva_frame: Option<u32>,
    /// 固定炮塔、扫车身朝向。
    pub sweep_body: Option<bool>,
    /// 固定车身、扫炮塔朝向。
    pub sweep_turret: Option<bool>,
    /// 固定朝向、扫 HVA 帧。
    pub sweep_hva: Option<bool>,
    /// `--sweep-hva` 时的帧数（`0..n`）；缺省 3。
    pub hva_frame_count: Option<u32>,
}

/// 单层诊断行（N-API）。
#[napi(object)]
pub struct MobileVxlLayerDiagJs {
    pub role: String,
    pub vxl_name: String,
    pub hva_name: String,
    pub vxl_hit: bool,
    pub hva_hit: bool,
    pub facing: Option<u32>,
    pub width: Option<u32>,
    pub height: Option<u32>,
    pub offset_x: Option<i32>,
    pub offset_y: Option<i32>,
    pub cell_offset_x: Option<i32>,
    pub cell_offset_y: Option<i32>,
    pub origin_px: Option<i32>,
    pub origin_py: Option<i32>,
}

/// 单次姿态诊断报告（N-API）。
#[napi(object)]
pub struct MobileVxlDiagReportJs {
    pub stem: String,
    pub body_facing: u32,
    pub turret_facing: u32,
    pub hva_frame: u32,
    pub layers: Vec<MobileVxlLayerDiagJs>,
    pub notes: Vec<String>,
}

/// 载具 VXL 分图层诊断结果（N-API）。
#[napi(object)]
pub struct DiagnoseMobileVxlResultJs {
    pub edition: String,
    pub stem: String,
    pub type_id: Option<String>,
    pub mounted_root: u32,
    pub mounted_nested: u32,
    pub reports: Vec<MobileVxlDiagReportJs>,
}

fn clamp_facing_byte(v: Option<u32>, default: u8) -> u8 {
    v.map(|n| (n & 0xFF) as u8).unwrap_or(default)
}

/// 对安装目录中的载具 VXL 做 body/turret/barrel/shadow 尺寸与原点诊断。
#[napi]
pub fn diagnose_mobile_vxl(options: DiagnoseMobileVxlOptions) -> Result<DiagnoseMobileVxlResultJs> {
    let path = PathBuf::from(options.path.trim());
    if path.as_os_str().is_empty() {
        return Err(Error::from_reason("diagnose-mobile-vxl: --path must not be empty"));
    }
    let body_facing = clamp_facing_byte(options.body_facing, 0);
    let turret_facing = clamp_facing_byte(options.turret_facing, body_facing);
    let req = DiagnoseMobileVxlRequest {
        ra2_dir: path,
        edition: options.edition.filter(|s| !s.trim().is_empty()),
        stem: options.stem.filter(|s| !s.trim().is_empty()),
        type_id: options.type_id.filter(|s| !s.trim().is_empty()),
        body_facing,
        turret_facing,
        hva_frame: options.hva_frame.unwrap_or(0),
        sweep_body: options.sweep_body.unwrap_or(false),
        sweep_turret: options.sweep_turret.unwrap_or(false),
        sweep_hva: options.sweep_hva.unwrap_or(false),
        hva_frame_count: options.hva_frame_count.filter(|&n| n > 0),
    };
    let result = diagnose_mobile_vxl_install(&req).map_err(|e| Error::from_reason(format!("{e}")))?;
    Ok(DiagnoseMobileVxlResultJs {
        edition: result.edition,
        stem: result.stem,
        type_id: result.type_id,
        mounted_root: result.mounted_root as u32,
        mounted_nested: result.mounted_nested as u32,
        reports: result
            .reports
            .into_iter()
            .map(|r| MobileVxlDiagReportJs {
                stem: r.stem,
                body_facing: u32::from(r.body_facing),
                turret_facing: u32::from(r.turret_facing),
                hva_frame: r.hva_frame,
                layers: r
                    .layers
                    .into_iter()
                    .map(|l| MobileVxlLayerDiagJs {
                        role: l.role,
                        vxl_name: l.vxl_name,
                        hva_name: l.hva_name,
                        vxl_hit: l.vxl_hit,
                        hva_hit: l.hva_hit,
                        facing: l.facing.map(u32::from),
                        width: l.width,
                        height: l.height,
                        offset_x: l.offset_x,
                        offset_y: l.offset_y,
                        cell_offset_x: l.cell_offset_x,
                        cell_offset_y: l.cell_offset_y,
                        origin_px: l.origin_px,
                        origin_py: l.origin_py,
                    })
                    .collect(),
                notes: r.notes,
            })
            .collect(),
    })
}
