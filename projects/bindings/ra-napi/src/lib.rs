//! N-API 入口：`version` + `launch` + `extract` + `unpack`。

#![deny(clippy::all)]

use std::path::PathBuf;

use napi::bindgen_prelude::*;
use napi_derive::napi;
use ra_config::LaunchOverride;
use ra_desktop::extract::{ExtractRequest, UnpackRequest, extract_named, unpack_all};

/// 绑定版本字符串。
#[napi]
pub fn version() -> String {
    env!("CARGO_PKG_VERSION").to_string()
}

/// `ra2 launch` 选项。
#[napi(object)]
pub struct LaunchOptions {
    /// 游戏安装根目录（含 MIX / INI）。
    pub path: String,
    /// 可选版本：`ra2` / `yr` 等。
    pub edition: Option<String>,
}

/// 注入路径覆盖并阻塞进入 GUI 事件循环。
#[napi]
pub fn launch(options: LaunchOptions) -> Result<()> {
    let path = PathBuf::from(options.path.trim());
    if path.as_os_str().is_empty() {
        return Err(Error::from_reason("--path must not be empty"));
    }
    ra_config::set_launch_override(LaunchOverride { ra2_dir: path, edition: options.edition.filter(|s| !s.trim().is_empty()) });
    ra_desktop::run().map_err(|e| Error::from_reason(format!("{e}")))
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
    /// 为 `.shp` 额外写出各帧 PNG。
    pub decode_shp: Option<bool>,
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
    /// SHP 已解码帧数（未解码则为 `null`）。
    pub shp_frames: Option<u32>,
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

/// 按逻辑名从安装 VFS 导出原始字节（可选 SHP→PNG）。
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
