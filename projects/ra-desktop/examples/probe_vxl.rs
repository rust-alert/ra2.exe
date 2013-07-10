//! 无窗口探测：零售 VXL / HVA / 炮塔层与朝向光栅。

use ra_adaptor::{detect_edition, find_ci_file};
use ra_assets::{HvaFile, MixVfs, Palette, VxlFile, rasterize_vxl, rasterize_vxl_layers, rasterize_vxl_posed};
use ra_types::{GameEdition, RaError, RaResult};
use std::path::{Path, PathBuf};

fn probe(root: &Path, edition: GameEdition) -> RaResult<()> {
    let manifest = detect_edition(root, Some(edition))?;
    let mut vfs = MixVfs::new();
    for name in &manifest.present_mixes {
        let path = find_ci_file(root, name).ok_or_else(|| RaError::MissingFile(name.clone()))?;
        let data = std::fs::read(&path).map_err(|e| RaError::Io(format!("{}: {e}", path.display())))?;
        let _ = vfs.mount_bytes(name.clone(), data);
    }
    for name in manifest.chain.nested_mix_files {
        let _ = vfs.mount_nested(name);
    }

    let pal = vfs.read("unittem.pal").and_then(|b| Palette::parse(&b).ok());

    let mut ok = 0usize;
    let mut fail = 0usize;
    let mut multi = 0usize;
    let mut turret = 0usize;
    let stems = [
        "taxi", "car", "bus", "mtnk", "htk", "sref", "orca", "1tnk", "2tnk", "3tnk", "4tnk", "htnk", "ltnk", "apoc", "harv",
        "dred", "carrier", "beag", "zep", "bfrt", "flak", "ttnk", "v3", "dtrk", "schp", "shad", "cmn2", "cmn3",
    ];
    for stem in stems {
        let file = format!("{stem}.vxl");
        let hva_name = format!("{stem}.hva");
        let Some(bytes) = vfs.read(&file)
        else {
            continue;
        };
        let has_tur = vfs.read(&format!("{stem}tur.vxl")).is_some();
        let has_barl = vfs.read(&format!("{stem}barl.vxl")).is_some() || vfs.read(&format!("{stem}barrel.vxl")).is_some();
        if has_tur || has_barl {
            turret += 1;
        }
        let hva = vfs.read(&hva_name).and_then(|b| HvaFile::parse(&b).ok());
        match VxlFile::parse(&bytes) {
            Ok(vxl) => {
                if vxl.limb_count > 1 {
                    multi += 1;
                }
                let limbs: Vec<String> = vxl
                    .limbs
                    .iter()
                    .map(|l| format!("{}:{}x{}x{}/v{}/s{:.4}", l.name, l.size_x, l.size_y, l.size_z, l.voxels.len(), l.scale))
                    .collect();
                let hva_info = match &hva {
                    Some(h) => format!("hva frames={} sections={}", h.frame_count, h.section_count),
                    None => "hva=miss".into(),
                };
                let raster = match &pal {
                    Some(p) => {
                        let base = rasterize_vxl(&vxl, p).map(|s| {
                            let opaque = s.rgba.chunks(4).filter(|c| c[3] > 0).count();
                            format!("{}x{} opaque={opaque}", s.width, s.height)
                        });
                        let facing96 =
                            rasterize_vxl_posed(&vxl, p, hva.as_ref(), 96).map(|s| format!("{}x{}", s.width, s.height));
                        let layered = if has_tur || has_barl {
                            let mut owned: Vec<(VxlFile, Option<HvaFile>)> = vec![(vxl.clone(), hva.clone())];
                            for suffix in ["tur", "barl", "barrel"] {
                                let Some(b) = vfs.read(&format!("{stem}{suffix}.vxl"))
                                else {
                                    continue;
                                };
                                let Ok(lv) = VxlFile::parse(&b)
                                else {
                                    continue;
                                };
                                let lh = vfs.read(&format!("{stem}{suffix}.hva")).and_then(|x| HvaFile::parse(&x).ok());
                                owned.push((lv, lh));
                                if suffix.starts_with("bar") {
                                    break;
                                }
                            }
                            let refs: Vec<_> = owned.iter().map(|(v, h)| (v, h.as_ref())).collect();
                            rasterize_vxl_layers(&refs, p, 0, 0)
                                .map(|s| format!(" layers={}x{}", s.width, s.height))
                                .unwrap_or_default()
                        }
                        else {
                            String::new()
                        };
                        match (base, facing96) {
                            (Some(b), Some(f)) => format!("{b} face96={f}{layered}"),
                            (Some(b), None) => format!("{b}{layered}"),
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
    eprintln!("summary ok={ok} fail={fail} multi_limb={multi} with_turret_layer={turret}");
    if ok == 0 {
        return Err(RaError::Msg("没有成功解析任何 VXL".into()));
    }
    Ok(())
}

fn main() {
    let mut args = std::env::args().skip(1);
    let Some(root) = args.next().map(PathBuf::from)
    else {
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
