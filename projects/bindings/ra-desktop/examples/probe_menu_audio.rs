//! 无窗口：探测菜单 BGM / 点击音效在安装包中的解析路径。

use ra_adaptor::{detect_edition, find_ci_file};
use ra_assets::{AudioIndex, IniDocument, MixVfs, decode_audio_bytes};
use ra_types::{GameEdition, RaError, RaResult};
use std::path::{Path, PathBuf};

fn mount_install(root: &Path, edition: Option<GameEdition>) -> RaResult<MixVfs> {
    let manifest = detect_edition(root, edition)?;
    let mut vfs = MixVfs::new();
    for name in &manifest.present_mixes {
        let Some(path) = find_ci_file(root, name)
        else {
            continue;
        };
        let data = std::fs::read(&path).map_err(|e| RaError::Io(format!("{}: {e}", path.display())))?;
        let _ = vfs.mount_bytes(name.clone(), data);
    }
    for name in manifest.chain.nested_mix_files {
        let _ = vfs.mount_nested_all_from_parents(name);
    }
    Ok(vfs)
}

fn main() {
    let mut args = std::env::args().skip(1);
    let Some(root) = args.next().map(PathBuf::from)
    else {
        eprintln!("用法: probe_menu_audio <游戏目录> [edition]");
        std::process::exit(2);
    };
    let edition = args.next().and_then(|s| GameEdition::parse(&s).ok());
    if let Err(e) = run(&root, edition) {
        eprintln!("probe_menu_audio failed: {e}");
        std::process::exit(1);
    }
}

fn run(root: &Path, edition: Option<GameEdition>) -> RaResult<()> {
    let vfs = mount_install(root, edition)?;

    let intro_sound = vfs
        .read("theme.ini")
        .as_deref()
        .and_then(|b| IniDocument::parse(b).ok())
        .as_ref()
        .and_then(|d| d.get("INTRO", "Sound"))
        .map(|s| s.trim().trim_start_matches(['$', '#']).to_string())
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| "Drok".into());
    println!("INTRO Sound={intro_sound}");
    println!(
        "theme.mix readable={} len={:?}",
        vfs.read("theme.mix").is_some(),
        vfs.read("theme.mix").map(|b| b.len())
    );

    // sound.ini：整文件可能解析失败，做宽松抽行
    if let Some(bytes) = vfs.read("sound.ini") {
        match IniDocument::parse(&bytes) {
            Ok(doc) => {
                for name in ["MenuClick", "MainButtonClick", "GenericClick", "MenuSlideIn"] {
                    println!(
                        "[{name}] Sounds={:?}",
                        doc.get(name, "Sounds")
                    );
                }
            }
            Err(e) => {
                println!("sound.ini parse_err={e}");
                let text = String::from_utf8_lossy(&bytes);
                for key in ["MenuClick", "UMENUCL", "MainButton", "GenericClick", "MenuSlide"] {
                    if let Some(idx) = text.find(key) {
                        let snip: String = text[idx..].chars().take(120).collect();
                        println!("snip@{key}: {}", snip.replace('\n', " | "));
                    }
                }
            }
        }
    }

    if let (Some(idx), Some(bag)) = (vfs.read("audio.idx"), vfs.read("audio.bag")) {
        let index = AudioIndex::parse(&idx, bag).expect("idx");
        // 最大的若干条目（可能是长采样）
        let mut entries: Vec<_> = index
            .names()
            .filter_map(|n| index.get(n).map(|e| (e.name.clone(), e.size)))
            .collect();
        entries.sort_by_key(|(_, s)| std::cmp::Reverse(*s));
        println!("bag largest 15:");
        for (n, s) in entries.iter().take(15) {
            println!("  {n} size={s}");
        }
        for n in ["UMENUCL1", "UMENUCL2", "UMENUCL3", "UMENUCLI", "UMENUCL"] {
            println!("decode {n}: {}", index.decode(n).is_some());
        }
        for e in index.names() {
            let u = e.to_ascii_uppercase();
            if u.contains("UMENU") || u.contains("SLIDE") || u.contains("GRIND") {
                println!("  name={e}");
            }
        }
    }

    for name in [
        format!("{intro_sound}.wav"),
        "Grinder.wav".into(),
        "grinder.wav".into(),
    ] {
        if let Some(b) = vfs.read(&name) {
            println!(
                "{name} decode={:?}",
                decode_audio_bytes(&b, Some("wav")).map(|p| p.samples.len())
            );
        }
    }
    Ok(())
}
