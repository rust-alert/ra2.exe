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
        Self {
            root,
            vfs: MixVfs::new(),
        }
    }
}

impl AssetSource for GameAssetSource {
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
