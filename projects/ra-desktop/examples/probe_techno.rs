//! 无窗口探测：rules TechnoType 数量与抽样。

use ra_adaptor::{detect_edition, find_ci_file};
use ra_assets::MixVfs;
use ra_rules::{load_rules, TechnoKind};
use ra_types::{AssetSource, GameEdition, RaError, RaResult};
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
    let db = load_rules(&source, edition)?;
    let t = &db.techno_types;
    eprintln!(
        "OK techno_types#{} infantry={} vehicle={} aircraft={} building={}",
        t.len(),
        t.count_kind(TechnoKind::Infantry),
        t.count_kind(TechnoKind::Vehicle),
        t.count_kind(TechnoKind::Aircraft),
        t.count_kind(TechnoKind::Building),
    );
    for id in ["MTNK", "E1", "GACNST", "ORCA", "HTNK"] {
        match t.get(id) {
            Some(tt) => eprintln!(
                "  {id}: str={} spd={} cost={} armor={} image={}",
                tt.strength, tt.speed, tt.cost, tt.armor, tt.image
            ),
            None => eprintln!("  {id}: miss"),
        }
    }
    Ok(())
}

fn main() {
    let mut args = std::env::args().skip(1);
    let Some(root) = args.next().map(PathBuf::from) else {
        eprintln!("用法: probe_techno <游戏目录> [edition]");
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
