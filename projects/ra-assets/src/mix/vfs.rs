//! 多层 MIX 虚拟目录：按挂载顺序查找，先命中者优先。

use ra_types::{RaError, RaResult};

use super::archive::MixArchive;

/// 已挂载 MIX 档案的虚拟文件系统。
#[derive(Debug, Default)]
pub struct MixVfs {
    archives: Vec<(String, MixArchive)>,
}

impl MixVfs {
    /// 创建空虚拟目录。
    pub fn new() -> Self {
        Self::default()
    }

    /// 挂载一份已解析档案（后挂载者仍排在查找链后方）。
    pub fn mount(&mut self, name: impl Into<String>, archive: MixArchive) {
        self.archives.push((name.into(), archive));
    }

    /// 已挂载档案数。
    pub fn archive_count(&self) -> usize {
        self.archives.len()
    }

    /// 按名取字节；返回拷贝，便于壳层持有。
    pub fn read(&self, name: &str) -> Option<Vec<u8>> {
        for (_, archive) in &self.archives {
            if let Some(bytes) = archive.get_by_name(name) {
                return Some(bytes.to_vec());
            }
        }
        None
    }

    /// 若某已挂载档案含嵌套 MIX，则解析并挂上。
    pub fn mount_nested(&mut self, name: &str) -> RaResult<bool> {
        let Some(bytes) = self.read(name)
        else {
            return Ok(false);
        };
        let archive = MixArchive::parse(bytes)?;
        self.mount(name, archive);
        Ok(true)
    }

    /// 从原始字节挂载顶层 MIX。
    pub fn mount_bytes(&mut self, name: impl Into<String>, data: Vec<u8>) -> RaResult<()> {
        let name = name.into();
        let archive = MixArchive::parse(data).map_err(|e| RaError::Parse(format!("挂载 {name} 失败: {e}")))?;
        self.mount(name, archive);
        Ok(())
    }
}
