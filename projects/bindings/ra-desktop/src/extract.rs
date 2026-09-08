//! 从已挂载安装目录导出资源（供 `ra2 extract` / `ra2 unpack` / N-API 使用）。
//!
//! - [`extract_named`]：按逻辑名导出（替代一次性 `probe_*`）。
//! - [`unpack_all`]：把已挂载 MIX 树中的**全部索引条目**写出（按档案分子目录，文件名为条目 id）。
//!
//! MIX 索引不含原文件名，全量解包只能以 `mix_hash` id 落盘；已知逻辑名请用 `extract`。

use std::path::{Path, PathBuf};

use ra_adaptor::detect_edition;
use ra_assets::{Palette, ShpFile};
use ra_renderer::RgbaImage;
use ra_types::{AssetSource, GameEdition, RaError, RaResult};

use crate::{
    config::{DesktopConfig, load_desktop_config_with_diagnostics},
    fs_source::{AssetOrigin, GameAssetSource},
};

/// 一次导出请求。
#[derive(Debug, Clone)]
pub struct ExtractRequest {
    /// 游戏安装根目录。
    pub ra2_dir: PathBuf,
    /// 可选版本；`None` 则自动探测。
    pub edition: Option<String>,
    /// 输出目录（不存在则创建）。
    pub out_dir: PathBuf,
    /// 要导出的逻辑文件名（如 `sdtp.shp`、`title.pcx`）。
    pub names: Vec<String>,
    /// SHP 解码用调色板逻辑名；缺省按 `shell.pal` → `unittem.pal` 尝试。
    pub palette: Option<String>,
    /// 为 `.shp` 额外写出 `name.frameNNNN.png`。
    pub decode_shp: bool,
}

/// 单个已写出文件。
#[derive(Debug, Clone)]
pub struct ExtractedFile {
    /// 请求的逻辑名。
    pub name: String,
    /// 落盘路径。
    pub path: PathBuf,
    /// 字节数。
    pub bytes: usize,
    /// 来源说明（loose / mix）。
    pub origin: String,
    /// 若为 SHP 且已解码，帧数。
    pub shp_frames: Option<usize>,
}

/// 导出结果摘要。
#[derive(Debug, Clone)]
pub struct ExtractReport {
    /// 成功写出的文件。
    pub written: Vec<ExtractedFile>,
    /// 不可读的逻辑名。
    pub missing: Vec<String>,
    /// 探测到的版本。
    pub edition: String,
    /// 成功挂载的根 MIX 数。
    pub mounted_root: usize,
    /// 成功挂载的嵌套 MIX 份数。
    pub mounted_nested: usize,
}

impl ExtractRequest {
    /// 用桌面配置补默认安装路径（仍须显式 `out_dir` / `names`）。
    pub fn from_desktop_defaults(out_dir: PathBuf, names: Vec<String>) -> Self {
        let (cfg, _) = load_desktop_config_with_diagnostics();
        Self::from_config(&cfg, out_dir, names)
    }

    /// 从桌面配置构造（覆盖 `out_dir` / `names`）。
    pub fn from_config(cfg: &DesktopConfig, out_dir: PathBuf, names: Vec<String>) -> Self {
        Self {
            ra2_dir: cfg.ra2_dir.clone(),
            edition: cfg.edition.clone(),
            out_dir,
            names,
            palette: None,
            decode_shp: false,
        }
    }
}

/// 按逻辑名导出；缺少的名字记入 `missing`，不因单项失败而中止。
pub fn extract_named(req: &ExtractRequest) -> RaResult<ExtractReport> {
    if req.names.is_empty() {
        return Err(RaError::Msg("extract: names must not be empty".into()));
    }
    if req.out_dir.as_os_str().is_empty() {
        return Err(RaError::Msg("extract: out_dir must not be empty".into()));
    }

    let explicit = match req.edition.as_deref() {
        Some(s) => Some(GameEdition::parse(s)?),
        None => None,
    };
    let manifest = detect_edition(&req.ra2_dir, explicit)?;
    let mut source = GameAssetSource::new(manifest.root.clone());
    let (mounted_root, _) = source.mount_root_plan(&manifest.composition.root_mount_plan);
    let (mounted_nested, _) = source.mount_nested_plan(&manifest.composition.nested_mount_plan);

    std::fs::create_dir_all(&req.out_dir).map_err(|e| RaError::Io(format!("{}: {e}", req.out_dir.display())))?;

    let mut written = Vec::new();
    let mut missing = Vec::new();

    for name in &req.names {
        let Some(hit) = source.resolve(name)
        else {
            missing.push(name.clone());
            continue;
        };
        let safe = sanitize_filename(name);
        let dest = req.out_dir.join(&safe);
        if let Some(parent) = dest.parent() {
            std::fs::create_dir_all(parent).map_err(|e| RaError::Io(format!("{}: {e}", parent.display())))?;
        }
        std::fs::write(&dest, &hit.bytes).map_err(|e| RaError::Io(format!("{}: {e}", dest.display())))?;

        let origin = match &hit.origin {
            AssetOrigin::Loose { path } => format!("loose:{}", path.display()),
            AssetOrigin::Mix { archive, parent, layer_id, priority } => match (parent, layer_id) {
                (Some(p), Some(l)) => format!("mix:{archive} parent={p} layer={l} prio={priority}"),
                (Some(p), None) => format!("mix:{archive} parent={p} prio={priority}"),
                (None, Some(l)) => format!("mix:{archive} layer={l} prio={priority}"),
                (None, None) => format!("mix:{archive} prio={priority}"),
            },
        };

        let mut shp_frames = None;
        if req.decode_shp && name.to_ascii_lowercase().ends_with(".shp") {
            match decode_shp_frames_to_png(&source, name, &hit.bytes, &req.out_dir, &safe, req.palette.as_deref()) {
                Ok(n) => shp_frames = Some(n),
                Err(e) => tracing::warn!(name = %name, "SHP 解码跳过 · {e}"),
            }
        }

        written.push(ExtractedFile {
            name: name.clone(),
            path: dest,
            bytes: hit.bytes.len(),
            origin,
            shp_frames,
        });
    }

    Ok(ExtractReport {
        written,
        missing,
        edition: manifest.chain.edition.as_str().to_string(),
        mounted_root,
        mounted_nested,
    })
}

fn sanitize_filename(name: &str) -> String {
    name.replace(['\\', '/', ':', '*', '?', '"', '<', '>', '|'], "_")
}

fn decode_shp_frames_to_png(
    source: &GameAssetSource,
    logical_name: &str,
    shp_bytes: &[u8],
    out_dir: &Path,
    safe_stem: &str,
    palette_override: Option<&str>,
) -> RaResult<usize> {
    let shp = ShpFile::parse(shp_bytes)?;
    let pal = load_palette_for_shp(source, logical_name, palette_override)?;
    let stem = Path::new(safe_stem).file_stem().and_then(|s| s.to_str()).unwrap_or(safe_stem);
    let mut decoded = 0usize;
    for (i, frame) in shp.frames.iter().enumerate() {
        if frame.frame_width == 0 || frame.frame_height == 0 {
            continue;
        }
        let rgba = frame.to_rgba(&pal);
        let Some(img) = RgbaImage::from_raw(u32::from(frame.frame_width), u32::from(frame.frame_height), rgba)
        else {
            continue;
        };
        let png = out_dir.join(format!("{stem}.frame{i:04}.png"));
        img.save(&png).map_err(|e| RaError::Io(format!("{}: {e}", png.display())))?;
        decoded += 1;
    }
    Ok(decoded)
}

fn load_palette_for_shp(source: &GameAssetSource, shp_name: &str, override_pal: Option<&str>) -> RaResult<Palette> {
    let mut candidates: Vec<String> = Vec::new();
    if let Some(p) = override_pal {
        candidates.push(p.to_string());
    }
    let lower = shp_name.to_ascii_lowercase();
    if let Some(stem) = Path::new(&lower).file_stem().and_then(|s| s.to_str()) {
        candidates.push(format!("{stem}.pal"));
    }
    candidates.push("shell.pal".into());
    candidates.push("shell2.pal".into());
    candidates.push("unittem.pal".into());

    let mut last = None;
    for name in candidates {
        match source.read(&name) {
            Ok(bytes) => match Palette::parse(&bytes) {
                Ok(pal) => return Ok(pal),
                Err(e) => last = Some(format!("{name}: {e}")),
            },
            Err(e) => last = Some(format!("{name}: {e}")),
        }
    }
    Err(RaError::Msg(format!(
        "no palette for {shp_name}: {}",
        last.unwrap_or_else(|| "no candidates".into())
    )))
}

/// 全量解包请求。
#[derive(Debug, Clone)]
pub struct UnpackRequest {
    /// 游戏安装根目录。
    pub ra2_dir: PathBuf,
    /// 可选版本；`None` 则自动探测。
    pub edition: Option<String>,
    /// 输出根目录。
    pub out_dir: PathBuf,
}

/// 全量解包结果。
#[derive(Debug, Clone)]
pub struct UnpackReport {
    /// 写出的条目数。
    pub files_written: usize,
    /// 累计字节。
    pub bytes_written: u64,
    /// 参与解包的挂载档案数。
    pub archives: usize,
    /// 探测版本。
    pub edition: String,
    /// 成功挂载的根 MIX 数。
    pub mounted_root: usize,
    /// 成功挂载的嵌套 MIX 份数。
    pub mounted_nested: usize,
    /// 输出根目录。
    pub out_dir: PathBuf,
}

/// 将安装内已挂载的全部 MIX 条目解包到 `out_dir`。
///
/// 目录布局：`out/<archive>/id_<XXXXXXXX>.bin`；嵌套档为 `out/<archive>@<parent>/...`。
/// 不解码 SHP/PCX；需要可读文件名时用 [`extract_named`]。
pub fn unpack_all(req: &UnpackRequest) -> RaResult<UnpackReport> {
    if req.out_dir.as_os_str().is_empty() {
        return Err(RaError::Msg("unpack: out_dir must not be empty".into()));
    }

    let explicit = match req.edition.as_deref() {
        Some(s) => Some(GameEdition::parse(s)?),
        None => None,
    };
    let manifest = detect_edition(&req.ra2_dir, explicit)?;
    let mut source = GameAssetSource::new(manifest.root.clone());
    let (mounted_root, _) = source.mount_root_plan(&manifest.composition.root_mount_plan);
    let (mounted_nested, _) = source.mount_nested_plan(&manifest.composition.nested_mount_plan);

    std::fs::create_dir_all(&req.out_dir).map_err(|e| RaError::Io(format!("{}: {e}", req.out_dir.display())))?;

    let mut files_written = 0usize;
    let mut bytes_written = 0u64;
    let mut archive_dirs = std::collections::BTreeSet::<String>::new();

    source.vfs.for_each_raw_entry(|entry| {
        let dir_name = match entry.parent {
            Some(parent) => format!("{}@{}", sanitize_filename(entry.archive_name), sanitize_filename(parent)),
            None => sanitize_filename(entry.archive_name),
        };
        archive_dirs.insert(dir_name.clone());
        let dest_dir = req.out_dir.join(&dir_name);
        if let Err(e) = std::fs::create_dir_all(&dest_dir) {
            tracing::warn!(dir = %dest_dir.display(), "创建解包目录失败 · {e}");
            return;
        }
        // 以无符号十六进制 id 命名；MIX 索引无原文件名。
        let file_name = format!("id_{:08X}.bin", entry.entry_id as u32);
        let dest = dest_dir.join(file_name);
        match std::fs::write(&dest, entry.bytes) {
            Ok(()) => {
                files_written += 1;
                bytes_written += entry.bytes.len() as u64;
            }
            Err(e) => tracing::warn!(path = %dest.display(), "写入解包文件失败 · {e}"),
        }
    });

    Ok(UnpackReport {
        files_written,
        bytes_written,
        archives: archive_dirs.len(),
        edition: manifest.chain.edition.as_str().to_string(),
        mounted_root,
        mounted_nested,
        out_dir: req.out_dir.clone(),
    })
}
