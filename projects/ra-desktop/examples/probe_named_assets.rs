//! 无窗口：挂载安装后，对给定逻辑文件名做可读性探测（不解码、不绘制）。
//!
//! 用于收集「填槽证据」：只有 `readable=true` 的名字才可写入 `ui_slots`。
//! 可读 ≠ 已进入 GPU UI pass，更 ≠ Pre-Alpha 视觉交付。

use ra_adaptor::{detect_edition, find_ci_file};
use ra_assets::MixVfs;
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
        let _ = vfs.mount_nested(name);
    }
    Ok(vfs)
}

fn main() {
    let mut args = std::env::args().skip(1);
    let Some(root) = args.next().map(PathBuf::from)
    else {
        eprintln!("用法: probe_named_assets <游戏目录> [edition] -- <name>...");
        eprintln!("示例: probe_named_assets D:/Games/RA2 ra2 -- mouse.shp ui.ini sdwrnanm.shp");
        std::process::exit(2);
    };

    let mut edition: Option<GameEdition> = None;
    let mut names: Vec<String> = Vec::new();
    let mut after_sep = false;
    for arg in args {
        if arg == "--" {
            after_sep = true;
            continue;
        }
        if !after_sep && edition.is_none() {
            match GameEdition::parse(&arg) {
                Ok(e) => {
                    edition = Some(e);
                    continue;
                }
                Err(_) => {
                    // 未识别为 edition 时当作文件名（兼容省略 edition）
                    names.push(arg);
                    after_sep = true;
                    continue;
                }
            }
        }
        names.push(arg);
    }

    if names.is_empty() {
        eprintln!("未提供待探测文件名（在 `--` 后列出）");
        std::process::exit(2);
    }

    if let Err(e) = run(&root, edition, &names) {
        eprintln!("probe_named_assets failed: {e}");
        std::process::exit(1);
    }
}

fn run(root: &Path, edition: Option<GameEdition>, names: &[String]) -> RaResult<()> {
    let vfs = mount_install(root, edition)?;
    let mut readable = 0usize;
    for name in names {
        let hit = vfs.read(name).is_some();
        if hit {
            readable += 1;
        }
        println!("{name}\treadable={hit}");
    }
    eprintln!("summary readable={}/{}", readable, names.len());
    Ok(())
}
