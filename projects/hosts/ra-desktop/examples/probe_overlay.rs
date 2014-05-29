//! 无窗口探测：解码地图 OverlayPack。

use ra_adaptor::{detect_edition, find_ci_file};
use ra_assets::MixVfs;
use ra_map::MapInfo;
use ra_types::{GameEdition, RaError, RaResult};
use std::path::{Path, PathBuf};

fn mount_edition(root: &Path, edition: GameEdition) -> RaResult<(PathBuf, MixVfs, usize)> {
    let manifest = detect_edition(root, Some(edition))?;
    let mut vfs = MixVfs::new();
    for name in &manifest.present_mixes {
        let path = find_ci_file(root, name).ok_or_else(|| RaError::MissingFile(name.clone()))?;
        let data = std::fs::read(&path).map_err(|e| RaError::Io(format!("{}: {e}", path.display())))?;
        if let Err(e) = vfs.mount_bytes(name.clone(), data) {
            eprintln!("skip {name}: {e}");
        }
    }
    let mut nested = 0usize;
    for name in manifest.chain.nested_mix_files {
        if matches!(vfs.mount_nested(name), Ok(true)) {
            nested += 1;
        }
    }
    Ok((manifest.root, vfs, nested))
}

fn probe(root: &Path, edition: GameEdition) -> RaResult<()> {
    let (_root, vfs, nested) = mount_edition(root, edition)?;
    const CANDIDATES: &[&str] = &["mp01t4.map", "mp03t4.map", "mp01t2.map", "mp02t4.map", "dustbowl.map", "goldst.map"];
    for name in CANDIDATES {
        let Some(bytes) = vfs.read(name)
        else {
            continue;
        };
        let map = MapInfo::parse_ini(edition, *name, &bytes)?;
        eprintln!(
            "OK {name} {}x{} {} iso#{} overlay#{} terrain#{} entities#{} wp#{} nested_mix={}",
            map.width,
            map.height,
            map.theater.as_str(),
            map.cells.len(),
            map.overlays.len(),
            map.terrain_objects.len(),
            map.entities.len(),
            map.waypoints.len(),
            nested
        );
        return Ok(());
    }
    Err(RaError::MissingFile("boot map candidates".into()))
}

fn main() {
    let mut args = std::env::args().skip(1);
    let Some(root) = args.next().map(PathBuf::from)
    else {
        eprintln!("用法: probe_overlay <游戏目录> [edition]");
        std::process::exit(2);
    };
    let edition = match args.next() {
        Some(s) => match GameEdition::parse(&s) {
            Ok(e) => e,
            Err(e) => {
                eprintln!("{e}");
                std::process::exit(2);
            }
        },
        None => GameEdition::Ra2,
    };
    if let Err(e) = probe(&root, edition) {
        eprintln!("probe failed: {e}");
        std::process::exit(1);
    }
}
