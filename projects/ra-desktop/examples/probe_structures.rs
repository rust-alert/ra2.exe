//! 无窗口探测：`[Structures]` → NewTheater / 普通 SHP。

use ra_adaptor::{detect_edition, find_ci_file};
use ra_assets::{IniDocument, MixVfs, ShpFile};
use ra_map::{
    new_theater_shp_name, theater_mix_names, MapEntityKind, MapInfo,
};
use ra_types::{GameEdition, RaError, RaResult};
use std::collections::HashSet;
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
    let bytes = vfs
        .read("mp01t4.map")
        .ok_or_else(|| RaError::MissingFile("mp01t4.map".into()))?;
    let map = MapInfo::parse_ini(edition, "mp01t4.map", &bytes)?;
    for mix in theater_mix_names(map.theater) {
        match vfs.mount_nested(mix) {
            Ok(true) => {}
            _ => {
                if let Some(path) = find_ci_file(root, mix) {
                    let data = std::fs::read(&path)
                        .map_err(|e| RaError::Io(format!("{}: {e}", path.display())))?;
                    let _ = vfs.mount_bytes(mix.to_string(), data);
                }
            }
        }
    }
    let art = IniDocument::parse(
        &vfs
            .read("art.ini")
            .ok_or_else(|| RaError::MissingFile("art.ini".into()))?,
    )?;

    let mut types = HashSet::new();
    let mut placed = 0usize;
    for e in &map.entities {
        if e.kind == MapEntityKind::Structure {
            types.insert(e.type_id.clone());
            placed += 1;
        }
    }
    let mut resolved = 0usize;
    for type_id in &types {
        let image = art.get(type_id, "Image").unwrap_or(type_id.as_str());
        let new_theater = art
            .get(type_id, "NewTheater")
            .is_some_and(|v| v.eq_ignore_ascii_case("yes"));
        let candidates = if new_theater {
            vec![
                new_theater_shp_name(image, map.theater),
                format!("{}.shp", image.to_ascii_lowercase()),
            ]
        } else {
            vec![
                format!("{}.shp", image.to_ascii_lowercase()),
                new_theater_shp_name(image, map.theater),
            ]
        };
        for file in candidates {
            if let Some(data) = vfs.read(&file) {
                if ShpFile::parse(&data).is_ok() {
                    resolved += 1;
                    break;
                }
            }
        }
    }
    eprintln!(
        "OK structures_placed={} types={} shp_types={} theater={}",
        placed,
        types.len(),
        resolved,
        map.theater.as_str()
    );
    Ok(())
}

fn main() {
    let mut args = std::env::args().skip(1);
    let Some(root) = args.next().map(PathBuf::from) else {
        eprintln!("用法: probe_structures <游戏目录> [edition]");
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
