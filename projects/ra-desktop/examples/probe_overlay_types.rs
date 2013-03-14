//! 无窗口探测：地图 OverlayPack id ↔ rules OverlayTypes 名。

use ra_adaptor::{detect_edition, find_ci_file};
use ra_assets::MixVfs;
use ra_map::MapInfo;
use ra_rules::load_rules;
use ra_types::{AssetSource, GameEdition, RaError, RaResult};
use std::collections::HashMap;
use std::path::{Path, PathBuf};

struct ProbeSource {
    root: PathBuf,
    vfs: MixVfs,
}

impl AssetSource for ProbeSource {
    fn read(&self, relative: &str) -> RaResult<Vec<u8>> {
        if let Some(path) = find_ci_file(&self.root, relative) {
            return std::fs::read(&path)
                .map_err(|e| RaError::Io(format!("{}: {e}", path.display())));
        }
        self.vfs
            .read(relative)
            .ok_or_else(|| RaError::MissingFile(relative.to_string()))
    }
}

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
    let source = ProbeSource {
        root: root.to_path_buf(),
        vfs,
    };
    let rules = load_rules(&source, edition)?;
    let bytes = source.read("mp01t4.map")?;
    let map = MapInfo::parse_ini(edition, "mp01t4.map", &bytes)?;

    let mut counts: HashMap<u8, usize> = HashMap::new();
    for cell in &map.overlays {
        *counts.entry(cell.overlay_id).or_default() += 1;
    }
    let mut ids: Vec<u8> = counts.keys().copied().collect();
    ids.sort_unstable();
    let mut named = 0usize;
    let mut unknown = 0usize;
    let mut sample = Vec::new();
    for id in &ids {
        let n = counts[id];
        match rules.overlay_types.name(*id) {
            Some(name) => {
                named += 1;
                if sample.len() < 12 {
                    sample.push(format!("{id}:{name}×{n}"));
                }
            }
            None => {
                unknown += 1;
                if sample.len() < 12 {
                    sample.push(format!("{id}:?×{n}"));
                }
            }
        }
    }

    eprintln!(
        "OK overlay_types#{} map_overlay_cells={} unique_ids={} named={} unknown={} sample=[{}]",
        rules.overlay_types.len(),
        map.overlays.len(),
        counts.len(),
        named,
        unknown,
        sample.join(", ")
    );
    Ok(())
}

fn main() {
    let mut args = std::env::args().skip(1);
    let Some(root) = args.next().map(PathBuf::from) else {
        eprintln!("用法: probe_overlay_types <游戏目录> [edition]");
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
