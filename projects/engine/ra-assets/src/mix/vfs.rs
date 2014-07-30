//! 多层 MIX 虚拟目录：按挂载优先级查找；高优先级覆盖低优先级，同优先级后挂载者覆盖。
//!
//! 嵌套包是**父档案的内容展开**，应继承父档的内容层优先级与层 id，
//! 不得统一降到固定「嵌套档」优先级。同名嵌套默认可从多个父来源各挂一份，
//! 由叶文件级 priority 决定覆盖（整包替换策略另议）。

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
    /// 若本档由某父 MIX 内嵌套打开，则为父档挂载名。
    parent: Option<String>,
    /// 内容层 id（来自挂载计划；嵌套继承父层）。
    layer_id: Option<String>,
}

/// 一次逻辑名解析的胜出说明（读与诊断应对齐此结果）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MixResolveHit<'a> {
    /// 胜出档案挂载名。
    pub archive_name: &'a str,
    /// 父档案名（根档为 `None`）。
    pub parent: Option<&'a str>,
    /// 内容层 id（若挂载时提供）。
    pub layer_id: Option<&'a str>,
    /// 胜出优先级。
    pub priority: i32,
    /// 条目字节。
    pub bytes: &'a [u8],
}

/// 挂载树上的一条原始索引条目（未做逻辑名解析）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MixRawEntry<'a> {
    /// 档案挂载名。
    pub archive_name: &'a str,
    /// 父档案名（根档为 `None`）。
    pub parent: Option<&'a str>,
    /// 内容层 id。
    pub layer_id: Option<&'a str>,
    /// 档案优先级。
    pub priority: i32,
    /// 条目 id（文件名 `mix_hash`）。
    pub entry_id: i32,
    /// 条目字节。
    pub bytes: &'a [u8],
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
        self.mount_with_meta(name, archive, 0, None, None);
    }

    /// 按显式优先级挂载档案。
    pub fn mount_with_priority(&mut self, name: impl Into<String>, archive: MixArchive, priority: i32) {
        self.mount_with_meta(name, archive, priority, None, None);
    }

    /// 挂载档案并记录父来源与内容层。
    pub fn mount_with_meta(
        &mut self,
        name: impl Into<String>,
        archive: MixArchive,
        priority: i32,
        parent: Option<String>,
        layer_id: Option<String>,
    ) {
        self.next_seq = self.next_seq.saturating_add(1);
        self.archives.push(MountedArchive { name: name.into(), archive, priority, seq: self.next_seq, parent, layer_id });
    }

    /// 已挂载档案数。
    pub fn archive_count(&self) -> usize {
        self.archives.len()
    }

    /// 遍历每个已挂载档案中的全部索引条目（含被更高优先级覆盖的同名哈希）。
    ///
    /// 用于全量解包；按逻辑名读取仍应走 [`Self::resolve_hit`]。
    pub fn for_each_raw_entry<F>(&self, mut visit: F)
    where
        F: FnMut(MixRawEntry<'_>),
    {
        for mounted in &self.archives {
            for entry in mounted.archive.entries() {
                let Some(bytes) = mounted.archive.get_by_id(entry.id)
                else {
                    continue;
                };
                visit(MixRawEntry {
                    archive_name: mounted.name.as_str(),
                    parent: mounted.parent.as_deref(),
                    layer_id: mounted.layer_id.as_deref(),
                    priority: mounted.priority,
                    entry_id: entry.id,
                    bytes,
                });
            }
        }
    }

    /// 按名取字节；返回拷贝，便于壳层持有。
    pub fn read(&self, name: &str) -> Option<Vec<u8>> {
        self.resolve_hit(name).map(|h| h.bytes.to_vec())
    }

    /// 解析逻辑名到「胜出档案名 + 字节」；无命中则 `None`。
    pub fn resolve(&self, name: &str) -> Option<(&str, &[u8])> {
        self.resolve_hit(name).map(|h| (h.archive_name, h.bytes))
    }

    /// 解析逻辑名并带上来源元数据。
    pub fn resolve_hit(&self, name: &str) -> Option<MixResolveHit<'_>> {
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
        Some(MixResolveHit {
            archive_name: mounted.name.as_str(),
            parent: mounted.parent.as_deref(),
            layer_id: mounted.layer_id.as_deref(),
            priority: mounted.priority,
            bytes,
        })
    }

    /// 若某已挂载档案含嵌套 MIX，则解析并挂上（继承该父档优先级）。
    ///
    /// 仅打开**当前全局胜出**的那一份嵌套字节。需要多来源文件覆盖时用
    /// [`Self::mount_nested_all_from_parents`]。
    pub fn mount_nested(&mut self, name: &str) -> RaResult<bool> {
        let Some(hit) = self.resolve_hit(name)
        else {
            return Ok(false);
        };
        // 已是展开后的同名嵌套档则不再打开。
        if hit.parent.is_some() && hit.archive_name.eq_ignore_ascii_case(name) {
            return Ok(false);
        }
        let parent = hit.archive_name.to_string();
        let priority = hit.priority;
        let layer_id = hit.layer_id.map(str::to_string);
        let bytes = hit.bytes.to_vec();
        if self.has_nested_from_parent(name, &parent) {
            return Ok(false);
        }
        let archive = MixArchive::parse(bytes)?;
        self.mount_with_meta(name, archive, priority, Some(parent), layer_id);
        Ok(true)
    }

    /// 兼容旧调用：忽略传入的固定嵌套优先级，改为继承父档（见 [`Self::mount_nested`]）。
    pub fn mount_nested_with_priority(&mut self, name: &str, _priority: i32) -> RaResult<bool> {
        self.mount_nested(name)
    }

    /// 从**每一个**已挂载父档中打开同名嵌套包（若有），各自继承该父档的 priority / layer。
    ///
    /// 返回新挂载份数。已存在「同父 + 同嵌套名」的不重复挂。
    pub fn mount_nested_all_from_parents(&mut self, nested_name: &str) -> RaResult<usize> {
        let mut jobs: Vec<(String, i32, Option<String>, Vec<u8>)> = Vec::new();
        for mounted in &self.archives {
            if mounted.name.eq_ignore_ascii_case(nested_name) {
                continue;
            }
            let Some(bytes) = mounted.archive.get_by_name(nested_name)
            else {
                continue;
            };
            if self.has_nested_from_parent(nested_name, &mounted.name) {
                continue;
            }
            if jobs.iter().any(|(p, _, _, _)| p.eq_ignore_ascii_case(&mounted.name)) {
                continue;
            }
            jobs.push((mounted.name.clone(), mounted.priority, mounted.layer_id.clone(), bytes.to_vec()));
        }

        let mut mounted_n = 0usize;
        for (parent, priority, layer_id, bytes) in jobs {
            if self.has_nested_from_parent(nested_name, &parent) {
                continue;
            }
            let archive = MixArchive::parse(bytes)?;
            self.mount_with_meta(nested_name, archive, priority, Some(parent), layer_id);
            mounted_n += 1;
        }
        Ok(mounted_n)
    }

    fn has_nested_from_parent(&self, nested_name: &str, parent: &str) -> bool {
        self.archives
            .iter()
            .any(|m| m.name.eq_ignore_ascii_case(nested_name) && m.parent.as_deref().is_some_and(|p| p.eq_ignore_ascii_case(parent)))
    }

    /// 从原始字节挂载顶层 MIX（优先级 `0`）。
    pub fn mount_bytes(&mut self, name: impl Into<String>, data: Vec<u8>) -> RaResult<()> {
        self.mount_bytes_with_priority(name, data, 0)
    }

    /// 从原始字节按显式优先级挂载顶层 MIX。
    pub fn mount_bytes_with_priority(&mut self, name: impl Into<String>, data: Vec<u8>, priority: i32) -> RaResult<()> {
        self.mount_bytes_with_meta(name, data, priority, None, None)
    }

    /// 从原始字节挂载并记录内容层。
    pub fn mount_bytes_with_meta(
        &mut self,
        name: impl Into<String>,
        data: Vec<u8>,
        priority: i32,
        parent: Option<String>,
        layer_id: Option<String>,
    ) -> RaResult<()> {
        let name = name.into();
        let archive = MixArchive::parse(data).map_err(|e| RaError::Parse(format!("挂载 {name} 失败: {e}")))?;
        self.mount_with_meta(name, archive, priority, parent, layer_id);
        Ok(())
    }
}
