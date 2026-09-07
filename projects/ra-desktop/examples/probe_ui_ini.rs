//! 无窗口：挂载安装目录并打印版本链 `ui.ini` 的 section / `.shp` 引用摘要。
//!
//! 零售 `ui.ini` 往往几乎不含菜单 SHP；本工具用于证伪「菜单目录在 ui.ini」并列出实际内容。

use ra_adaptor::{detect_edition, find_ci_file};
use ra_assets::{IniDocument, MixVfs};
use ra_types::{GameEdition, RaError, RaResult};
use std::path::{Path, PathBuf};

fn mount_install(root: &Path, edition: Option<GameEdition>) -> RaResult<(MixVfs, &'static str)> {
    let manifest = detect_edition(root, edition)?;
    let mut vfs = MixVfs::new();
    for name in &manifest.present_mixes {
        let Some(path) = find_ci_file(root, name)
        else {
            continue;
        };
        let data = std::fs::read(&path).map_err(|e| RaError::Io(format!("{}: {e}", path.display())))?;
        let _ = vfs.mount_bytes(name.clone(), data);
    }
    for name in manifest.chain.nested_mix_files {
        let _ = vfs.mount_nested(name);
    }
    Ok((vfs, manifest.chain.ui_ini))
}

fn main() {
    let mut args = std::env::args().skip(1);
    let Some(root) = args.next().map(PathBuf::from)
    else {
        eprintln!("用法: probe_ui_ini <游戏目录> [edition]");
        std::process::exit(2);
    };
    let edition = match args.next() {
        Some(s) => match GameEdition::parse(&s) {
            Ok(e) => Some(e),
            Err(e) => {
                eprintln!("{e}");
                std::process::exit(2);
            }
        },
        None => None,
    };
    if let Err(e) = run(&root, edition) {
        eprintln!("probe_ui_ini failed: {e}");
        std::process::exit(1);
    }
}

fn run(root: &Path, edition: Option<GameEdition>) -> RaResult<()> {
    let (vfs, ui_name) = mount_install(root, edition)?;
    let bytes = vfs.read(ui_name).ok_or_else(|| RaError::MissingFile(ui_name.into()))?;
    let doc = IniDocument::parse(&bytes)?;
    eprintln!("OK {ui_name} bytes={} sections={}", bytes.len(), doc.sections.len());
    for sec in &doc.sections {
        eprintln!("  [{}] entries={}", sec.name_raw, sec.entries.len());
        for (k, v) in sec.pairs() {
            eprintln!("    {k}={v}");
        }
    }
    let refs = doc.collect_shp_refs();
    eprintln!("shp_refs={}", refs.len());
    for name in &refs {
        let hit = vfs.read(name).is_some();
        eprintln!("  {name} readable={hit}");
    }
    if refs.is_empty() {
        eprintln!("hint: 版本链 ui.ini 未列出菜单 SHP，主菜单素材需另建页面资源模型");
    }
    Ok(())
}
