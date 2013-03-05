//! 无窗口探测：地形物件 SHP 是否能按剧院扩展名解出。

use ra_adaptor::{detect_edition, find_ci_file};
use ra_assets::{IniDocument, MixVfs, Palette, ShpFile};
use ra_map::{theater_mix_names, theater_tmp_extension, MapInfo};
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

    let art = vfs
        .read("art.ini")
        .and_then(|b| IniDocument::parse(&b).ok());
    let pal = vfs
        .read("unittem.pal")
        .ok_or_else(|| RaError::MissingFile("unittem.pal".into()))?;
    let pal = Palette::parse(&pal)?;
    let ext = theater_tmp_extension(map.theater);

    let mut unique: HashSet<String> = HashSet::new();
    for t in &map.terrain_objects {
        unique.insert(t.name.clone());
    }
    let mut resolved = 0usize;
    let mut frames = 0usize;
    for name in &unique {
        let image = art
            .as_ref()
            .and_then(|a| a.get(name, "Image"))
            .unwrap_or(name.as_str());
        let file = format!("{}.{ext}", image.to_ascii_lowercase());
        let Some(data) = vfs.read(&file) else {
            continue;
        };
        let Ok(shp) = ShpFile::parse(&data) else {
            continue;
        };
        resolved += 1;
        if let Some(frame) = shp.frames.first() {
            let rgba = frame.to_rgba(&pal);
            if rgba.len() == frame.frame_width as usize * frame.frame_height as usize * 4 {
                frames += 1;
            }
        }
    }

    eprintln!(
        "OK terrain_objs={} unique={} shp_types={} rgba_ok={} theater={}",
        map.terrain_objects.len(),
        unique.len(),
        resolved,
        frames,
        map.theater.as_str()
    );
    Ok(())
}

fn main() {
    let mut args = std::env::args().skip(1);
    let Some(root) = args.next().map(PathBuf::from) else {
        eprintln!("用法: probe_terrain_shp <游戏目录> [edition]");
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
