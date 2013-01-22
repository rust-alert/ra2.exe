//! 原生文件系统版 `AssetSource`。

use std::path::PathBuf;

use ra_types::{AssetSource, RaError, RaResult};

pub struct FsAssetSource {
    pub root: PathBuf,
}

impl AssetSource for FsAssetSource {
    fn read(&self, relative: &str) -> RaResult<Vec<u8>> {
        let path = self.root.join(relative);
        std::fs::read(&path).map_err(|e| RaError::Io(format!("{}: {e}", path.display())))
    }
}
