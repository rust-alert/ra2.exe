//! 多层 MIX 虚拟目录：按挂载优先级查找；高优先级覆盖低优先级，同优先级后挂载者覆盖。

use ra_types::{RaError, RaResult};

use super::archive::MixArchive;

#[derive(Debug)]
struct MountedArchive {
    name: String,
    archive: MixArchive,
    /// 资源层优先级（越大越优先）。
    priority: i32,
    /// 同优先级内的挂载序号（越大越优先）。
    seq: u32,
}

/// 已挂载 MIX 档案的虚拟文件系统。
#[derive(Debug, Default)]
pub struct MixVfs {
    archives: Vec<MountedArchive>,
    next_seq: u32,
}

impl MixVfs {
    /// 创建空虚拟目录。
    pub fn new() -> Self {
        Self::default()
    }

    /// 以优先级 `0` 挂载档案（同优先级内后挂载覆盖先挂载）。
    pub fn mount(&mut self, name: impl Into<String>, archive: MixArchive) {
        self.mount_with_priority(name, archive, 0);
    }

    /// 按显式优先级挂载档案。
    pub fn mount_with_priority(&mut self, name: impl Into<String>, archive: MixArchive, priority: i32) {
        self.next_seq = self.next_seq.saturating_add(1);
        self.archives.push(MountedArchive {
            name: name.into(),
            archive,
            priority,
            seq: self.next_seq,
        });
    }

    /// 已挂载档案数。
    pub fn archive_count(&self) -> usize {
        self.archives.len()
    }

    /// 按名取字节；返回拷贝，便于壳层持有。
    pub fn read(&self, name: &str) -> Option<Vec<u8>> {
        self.resolve(name).map(|(_, bytes)| bytes.to_vec())
    }

    /// 解析逻辑名到「胜出档案名 + 字节」；无命中则 `None`。
    pub fn resolve(&self, name: &str) -> Option<(&str, &[u8])> {
        let mut best: Option<(i32, u32, usize)> = None;
        for (idx, mounted) in self.archives.iter().enumerate() {
            if mounted.archive.get_by_name(name).is_none() {
                continue;
            }
            let key = (mounted.priority, mounted.seq);
            match best {
                None => best = Some((key.0, key.1, idx)),
                Some((bp, bs, _)) if key > (bp, bs) => best = Some((key.0, key.1, idx)),
                _ => {}
            }
        }
        let (_, _, idx) = best?;
        let mounted = &self.archives[idx];
        let bytes = mounted.archive.get_by_name(name)?;
        Some((mounted.name.as_str(), bytes))
    }

    /// 若某已挂载档案含嵌套 MIX，则解析并以优先级 `0` 挂上。
    pub fn mount_nested(&mut self, name: &str) -> RaResult<bool> {
        self.mount_nested_with_priority(name, 0)
    }

    /// 若某已挂载档案含嵌套 MIX，则解析并以给定优先级挂上。
    pub fn mount_nested_with_priority(&mut self, name: &str, priority: i32) -> RaResult<bool> {
        let Some(bytes) = self.read(name)
        else {
            return Ok(false);
        };
        let archive = MixArchive::parse(bytes)?;
        self.mount_with_priority(name, archive, priority);
        Ok(true)
    }

    /// 从原始字节挂载顶层 MIX（优先级 `0`）。
    pub fn mount_bytes(&mut self, name: impl Into<String>, data: Vec<u8>) -> RaResult<()> {
        self.mount_bytes_with_priority(name, data, 0)
    }

    /// 从原始字节按显式优先级挂载顶层 MIX。
    pub fn mount_bytes_with_priority(
        &mut self,
        name: impl Into<String>,
        data: Vec<u8>,
        priority: i32,
    ) -> RaResult<()> {
        let name = name.into();
        let archive =
            MixArchive::parse(data).map_err(|e| RaError::Parse(format!("挂载 {name} 失败: {e}")))?;
        self.mount_with_priority(name, archive, priority);
        Ok(())
    }
}
