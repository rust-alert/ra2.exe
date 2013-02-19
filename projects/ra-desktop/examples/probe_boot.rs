//! 无窗口探测：挂载零售目录并加载 rules。

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

fn probe(root: &Path) -> RaResult<()> {
    let manifest = detect_edition(root, Some(GameEdition::Ra2))?;
    let mut vfs = MixVfs::new();
    for name in &manifest.present_mixes {
        let path = find_ci_file(root, name)
            .ok_or_else(|| RaError::MissingFile(name.clone()))?;
        let data = std::fs::read(&path)
            .map_err(|e| RaError::Io(format!("{}: {e}", path.display())))?;
        eprintln!("mount {} ({} MiB)", name, data.len() / (1024 * 1024));
        if let Err(e) = vfs.mount_bytes(name.clone(), data) {
            eprintln!("skip {name}: {e}");
        }
    }
    let mut nested = 0usize;
    for name in manifest.chain.nested_mix_files {
        match vfs.mount_nested(name) {
            Ok(true) => {
                nested += 1;
                eprintln!("nested {name}");
            }
            Ok(false) => {}
            Err(e) => eprintln!("skip nested {name}: {e}"),
        }
    }
    let source = ProbeSource {
        root: root.to_path_buf(),
        vfs,
    };
    let rules = load_rules(&source, GameEdition::Ra2)?;
    eprintln!(
        "OK archives_nested={nested} rules_sections={} art_sections={}",
        rules.rules.sections.len(),
        rules.art.sections.len()
    );
    Ok(())
}

fn main() {
    let Some(root) = std::env::args().nth(1).map(PathBuf::from) else {
        eprintln!("用法: probe_boot <游戏目录>");
        std::process::exit(2);
    };
    if let Err(e) = probe(&root) {
        eprintln!("probe failed: {e}");
        std::process::exit(1);
    }
}
