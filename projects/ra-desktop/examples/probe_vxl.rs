//! 无窗口探测：零售 VXL / HVA 与朝向光栅。

use ra_adaptor::{detect_edition, find_ci_file};
use ra_assets::{rasterize_vxl, rasterize_vxl_posed, HvaFile, MixVfs, Palette, VxlFile};
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

    let pal = vfs
        .read("unittem.pal")
        .and_then(|b| Palette::parse(&b).ok());

    let mut ok = 0usize;
    let mut fail = 0usize;
    let mut multi = 0usize;
    let stems = [
        "taxi", "car", "bus", "mtnk", "htk", "sref", "orca", "1tnk", "2tnk", "3tnk", "4tnk",
        "htnk", "ltnk", "apoc", "harv", "dred", "carrier", "beag", "zep", "bfrt", "flak",
        "ttnk", "v3", "dtrk", "schp", "shad", "cmn2", "cmn3",
    ];
    for stem in stems {
        let file = format!("{stem}.vxl");
        let hva_name = format!("{stem}.hva");
        let Some(bytes) = vfs.read(&file) else {
            continue;
        };
        let hva = vfs
            .read(&hva_name)
            .and_then(|b| HvaFile::parse(&b).ok());
        match VxlFile::parse(&bytes) {
            Ok(vxl) => {
                if vxl.limb_count > 1 {
                    multi += 1;
                }
                let limbs: Vec<String> = vxl
                    .limbs
                    .iter()
                    .map(|l| {
                        format!(
                            "{}:{}x{}x{}/v{}",
                            l.name, l.size_x, l.size_y, l.size_z, l.voxels.len()
                        )
                    })
                    .collect();
                let hva_info = match &hva {
                    Some(h) => format!(
                        "hva frames={} sections={}",
                        h.frame_count, h.section_count
                    ),
                    None => "hva=miss".into(),
                };
                let raster = match &pal {
                    Some(p) => {
                        let base = rasterize_vxl(&vxl, p).map(|s| {
                            let opaque = s.rgba.chunks(4).filter(|c| c[3] > 0).count();
                            format!("{}x{} opaque={opaque}", s.width, s.height)
                        });
                        let facing96 = rasterize_vxl_posed(&vxl, p, hva.as_ref(), 96)
                            .map(|s| format!("{}x{}", s.width, s.height));
                        match (base, facing96) {
                            (Some(b), Some(f)) => format!("{b} face96={f}"),
                            (Some(b), None) => b,
                            _ => "raster=none".into(),
                        }
                    }
                    None => "raster=no-pal".into(),
                };
                eprintln!(
                    "OK {file} limbs={} voxels={} {hva_info} raster={raster} [{}]",
                    vxl.limb_count,
                    vxl.total_voxels(),
                    limbs.join(", ")
                );
                ok += 1;
            }
            Err(e) => {
                eprintln!("FAIL {file}: {e}");
                fail += 1;
            }
        }
    }
    eprintln!("summary ok={ok} fail={fail} multi_limb={multi}");
    if ok == 0 {
        return Err(RaError::Msg("没有成功解析任何 VXL".into()));
    }
    Ok(())
}

fn main() {
    let mut args = std::env::args().skip(1);
    let Some(root) = args.next().map(PathBuf::from) else {
        eprintln!("用法: probe_vxl <游戏目录> [edition]");
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
