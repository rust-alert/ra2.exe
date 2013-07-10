//! 磁盘松散文件 + 已挂载 MIX 的组合 `AssetSource`。

use std::path::PathBuf;

use ra_adaptor::find_ci_file;
use ra_assets::MixVfs;
use ra_types::{AssetSource, RaError, RaResult};

pub struct GameAssetSource {
    pub root: PathBuf,
    pub vfs: MixVfs,
}

impl GameAssetSource {
    pub fn new(root: PathBuf) -> Self {
        Self { root, vfs: MixVfs::new() }
    }

    /// 挂载清单中已存在的根 MIX。返回 `(成功数, 解析跳过数)`。
    pub fn mount_present_roots(&mut self, present_mixes: &[String]) -> (usize, usize) {
        let mut mounted = 0usize;
        let mut skipped = 0usize;
        for name in present_mixes {
            let Some(path) = find_ci_file(&self.root, name)
            else {
                continue;
            };
            let Ok(data) = std::fs::read(&path)
            else {
                skipped += 1;
                continue;
            };
            match self.vfs.mount_bytes(name.clone(), data) {
                Ok(()) => mounted += 1,
                Err(_) => skipped += 1,
            }
        }
        (mounted, skipped)
    }

    /// 尝试挂载嵌套 MIX 名列表。返回成功挂载数。
    pub fn mount_nested_names(&mut self, names: &[&str]) -> usize {
        let mut mounted = 0usize;
        for name in names {
            match self.vfs.mount_nested(name) {
                Ok(true) => mounted += 1,
                Ok(false) | Err(_) => {}
            }
        }
        mounted
    }
}

impl AssetSource for GameAssetSource {
    fn read(&self, relative: &str) -> RaResult<Vec<u8>> {
        if let Some(path) = find_ci_file(&self.root, relative) {
            return std::fs::read(&path).map_err(|e| RaError::Io(format!("{}: {e}", path.display())));
        }
        self.vfs.read(relative).ok_or_else(|| RaError::MissingFile(relative.to_string()))
    }
}
