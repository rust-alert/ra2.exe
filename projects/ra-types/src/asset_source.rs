//! 平台 I/O 边界：由壳实现；解析器只看见字节。

use crate::error::RaResult;

/// 读取游戏文件，避免把文件系统访问写进共享 crate。
pub trait AssetSource {
    fn read(&self, relative: &str) -> RaResult<Vec<u8>>;

    fn exists(&self, relative: &str) -> bool {
        self.read(relative).is_ok()
    }
}
