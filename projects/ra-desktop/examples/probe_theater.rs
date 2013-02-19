//! 探测剧院 MIX 与地图包条目。

use ra_adaptor::{detect_edition, find_ci_file};
use ra_assets::MixVfs;
use ra_types::{GameEdition, RaError, RaResult};
use std::path::{Path, PathBuf};

fn mount(root: &Path) -> RaResult<MixVfs> {
    let manifest = detect_edition(root, Some(GameEdition::Ra2))?;
    let mut vfs = MixVfs::new();
    for name in &manifest.present_mixes {
        let Some(path) = find_ci_file(root, name) else {
            continue;
        };
        let data = std::fs::read(&path)
            .map_err(|e| RaError::Io(format!("{}: {e}", path.display())))?;
        let _ = vfs.mount_bytes(name.clone(), data);
    }
    for name in manifest.chain.nested_mix_files {
        let _ = vfs.mount_nested(name);
    }
    Ok(vfs)
}

fn main() {
    let Some(root) = std::env::args().nth(1).map(PathBuf::from) else {
        eprintln!("用法: probe_theater <游戏目录>");
        std::process::exit(2);
    };
    let vfs = mount(&root).expect("mount");
    for name in [
        "temperat.mix",
        "snow.mix",
        "urban.mix",
        "lunar.mix",
        "desert.mix",
        "tem.mix",
        "sno.mix",
        "urb.mix",
        "isotem.pal",
        "isosno.pal",
        "isourb.pal",
    ] {
        eprintln!(
            "{name} => {}",
            vfs.read(name).map(|b| b.len()).unwrap_or(0)
        );
    }
    // maps01 is already mounted as root; try nested map names
    for name in ["amazon.map", "arena.map", "mp01t4.map", "c1m1.map", "sow.map"] {
        eprintln!(
            "{name} => {}",
            vfs.read(name).map(|b| b.len()).unwrap_or(0)
        );
    }
}
