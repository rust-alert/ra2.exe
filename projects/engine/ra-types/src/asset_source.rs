//! 平台 I/O 边界：由壳实现；解析器只看见字节。

use crate::error::RaResult;

/// 读取游戏文件，避免把文件系统访问写进共享 crate。
pub trait AssetSource {
    /// 按相对名读取完整字节。
    fn read(&self, relative: &str) -> RaResult<Vec<u8>>;

    /// 默认：能 `read` 成功即视为存在。
    fn exists(&self, relative: &str) -> bool {
        self.read(relative).is_ok()
    }
}
