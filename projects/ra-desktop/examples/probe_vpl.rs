//! 无窗口探测：零售 `voxels.vpl` 页数与抽样 remap。

use ra_adaptor::{detect_edition, find_ci_file};
use ra_assets::{MixVfs, VplFile};
use ra_types::{GameEdition, RaError, RaResult};
use std::path::{Path, PathBuf};

fn probe(root: &Path, edition: GameEdition) -> RaResult<()> {
    let manifest = detect_edition(root, Some(edition))?;
    let mut vfs = MixVfs::new();
    for name in &manifest.present_mixes {
        let path = find_ci_file(root, name)
            .ok_or_else(|| RaError::MissingFile(name.clone()))?;
        let data = std::fs::read(&path)
            .map_err(|e| RaError::Io(format!("{}: {e}", path.display())))?;
        let _ = vfs.mount_bytes(name.clone(), data);
    }
    for name in manifest.chain.nested_mix_files {
        let _ = vfs.mount_nested(name);
    }

    let Some(bytes) = vfs.read("voxels.vpl") else {
        return Err(RaError::MissingFile("voxels.vpl".into()));
    };
    let vpl = VplFile::parse(&bytes)?;
    eprintln!(
        "OK voxels.vpl pages={} first_remap={} last_remap={} size={}",
        vpl.page_count(),
        vpl.first_remap,
        vpl.last_remap,
        bytes.len()
    );
    for (page, color) in [(0u8, 16u8), (0, 100), (16, 16), (32, 200)] {
        let out = vpl.remap_color(page, color);
        eprintln!("  remap page={page} color={color} → {out}");
    }
    Ok(())
}

fn main() {
    let mut args = std::env::args().skip(1);
    let Some(root) = args.next().map(PathBuf::from) else {
        eprintln!("用法: probe_vpl <游戏目录> [edition]");
        std::process::exit(2);
    };
    let edition = match args.next() {
        Some(s) => GameEdition::parse(&s).unwrap_or(GameEdition::Ra2),
        None => GameEdition::Ra2,
    };
    if let Err(e) = probe(&root, edition) {
        eprintln!("probe failed: {e}");
        std::process::exit(1);
    }
}
