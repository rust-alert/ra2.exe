//! 从已挂载 MIX 解析一帧 SHP + 调色板。

use ra_adaptor::{detect_edition, find_ci_file};
use ra_assets::{MixVfs, Palette, ShpFile};
use ra_types::{GameEdition, RaError, RaResult};
use std::path::{Path, PathBuf};

fn mount_retail(root: &Path) -> RaResult<MixVfs> {
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
        eprintln!("用法: probe_shp <游戏目录>");
        std::process::exit(2);
    };
    if let Err(e) = run(&root) {
        eprintln!("probe_shp failed: {e}");
        std::process::exit(1);
    }
}

fn run(root: &Path) -> RaResult<()> {
    let vfs = mount_retail(root)?;
    let pal_name = "unittem.pal";
    let candidates = [
        "e1.shp",
        "ggun.shp",
        "mouse.shp",
        "clock.shp",
        "power.shp",
        "gaairc.shp",
    ];
    let pal_bytes = vfs
        .read(pal_name)
        .ok_or_else(|| RaError::MissingFile(pal_name.into()))?;
    let mut shp_name = "";
    let mut shp_bytes = None;
    for name in candidates {
        if let Some(bytes) = vfs.read(name) {
            shp_name = name;
            shp_bytes = Some(bytes);
            break;
        }
    }
    let shp_bytes =
        shp_bytes.ok_or_else(|| RaError::MissingFile("common shp candidates".into()))?;
    let pal = Palette::parse(&pal_bytes)?;
    let shp = ShpFile::parse(&shp_bytes)?;
    let frame0 = shp
        .frames
        .first()
        .ok_or_else(|| RaError::Parse("shp 无帧".into()))?;
    let rgba = frame0.to_rgba(&pal);
    eprintln!(
        "OK {shp_name} {}x{} frames={} frame0={}x{} rgba_bytes={} pal0_a={}",
        shp.width,
        shp.height,
        shp.frame_count(),
        frame0.frame_width,
        frame0.frame_height,
        rgba.len(),
        pal.colors[0].a
    );
    Ok(())
}
