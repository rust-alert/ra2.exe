//! 无窗口探测：rules `[Colors]` HSV 与阵营 `Color=`。

use ra_adaptor::{detect_edition, find_ci_file};
use ra_assets::MixVfs;
use ra_rules::load_rules;
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
    if let Some(sec) = db.rules.sections.get("Colors") {
        eprintln!("[Colors] entries={}", sec.order.len());
        for (i, (k, v)) in sec.order.iter().take(12).enumerate() {
            eprintln!("  {i}: {k}={v}");
        }
    } else {
        eprintln!("MISS [Colors]");
    }
    for house in [
        "Americans",
        "Alliance",
        "French",
        "Germans",
        "British",
        "Africans",
        "Arabs",
        "Confederation",
        "Russians",
        "Neutral",
        "Special",
    ] {
        if db.rules.sections.contains_key(house) {
            let color = db.rules.get(house, "Color").unwrap_or("?");
            let side = db.rules.get(house, "Side").unwrap_or("?");
            eprintln!("  house {house} Color={color} Side={side}");
        }
    }
    Ok(())
}

fn main() {
    let mut args = std::env::args().skip(1);
    let Some(root) = args.next().map(PathBuf::from) else {
        eprintln!("用法: probe_colors <游戏目录> [edition]");
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
